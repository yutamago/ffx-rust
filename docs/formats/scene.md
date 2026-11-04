# The field scene: what PhyreEngine draws, and from what

This is the round-1 census of M4 (`docs/rewrite-plan.md`): one field frame's
scene draws, each traced back to the cluster objects it uses, to its pass
and shader family, and to the source of every constant it reads. It is
the spec for round 2's Rust scene builder. Crate:
[`crates/ffx-scene`](../../crates/ffx-scene).

The frame is field-walk-1's (`tools/replays/field-walk.txt`): the salvage
ship deck `cdsp0000`, original code (`--hook-disable replacements`), ctrl
work 0's frame count 101, 197 draws in all. The reference captures are
`~/ffx-work/m4/captures/fw{1,2,3}-{a,b}` (never in git); the numbers below
are from `fw1-a`, and `fw3-b` and a capture of our own give the same.

```bash
# the census: one line per scene draw, then the totals and the rule checks
FFX_GAME_DIR=~/steam-games/FFX-X2-HD FFX_SCENE_MODS=~/ffx-work/instances/1/mods \
  cargo run --release -p ffx-scene --example census -- ~/ffx-work/m4/captures/fw1-a/capture.ffxgfx
# the gate (skips without FFX_GAME_DIR and FFX_SCENE_CAPTURE)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD FFX_SCENE_CAPTURE=~/ffx-work/m4/captures/fw1-a/capture.ffxgfx \
  FFX_SCENE_MODS=~/ffx-work/instances/1/mods cargo test --release -p ffx-scene
```

## What the captured game ran with

Two things outside the archive shape every capture the harness takes:

- **The user's mods.** `tools/instance.sh` links the game folder's `mods/`
  into every harness instance, and `ffx-hook`'s `mods.rs` serves those files
  in place of the archive's. On 2026-10-24 the folder holds 16 character
  mods; this frame's Tidus (`chr/pc/c001` model and texture) comes from
  "Tidus - LPv2. HiFi Low Poly Model", not from `FFX_Data.vbf`. Original
  and Rust runs load the same mods, so replays still compare like with
  like, but "the original's output" means "with the user's mods".
- **`GameSetting.ini`.** The harness copies the user's settings into its
  sandbox (only the resolution and volume are changed): `Quality=VQ_LOW`,
  `TextureQuality=TQ_LOW`, `MSAA=0`, `Shadow=0`, `HDAO=Off`. Texture
  quality decides which mip levels the game uploads (below), MSAA which
  pass names the map uses.

## Passes

`PhyreScene_render` (`0x00668930`) draws the 3D field into two colour
targets (colour, view-space normal) with one depth buffer. It clears them
first: colour from `*(scene+0xcc)+0x30..` (here (0.235, 0.235, 0.275, 1)),
depth 1 with flags 0xb, the normal target to (1, 1, 1, 1). Then
`Draw3DWorld` (`0x0064fbb0`) renders three `PWorld`s, setting the pass
(`*(scene+0xd0)+0x48`, a pointer to a static name: Opaque `0x00c169b4`,
Transparent `0x00c169d4`, OpaqueNoMSAA `0x00c169e4`, TransparentNoMSAA
`0x00c169f4`) before each `PApplication_renderWorld(world, camera, dir)`
(`0x0062be70`; dir 1 opaque, -1 transparent):

| World | Holds |
|---|---|
| scene `+0xac` | the 3D map (`PhyreScene_fieldMapLoad` `0x0065cbf0`) |
| scene `+0x7c` | the characters (cluster `+0x11e4`, moved there by `attachChrClusterTo3DWorld`), and cluster `+0x28c4` (ground shadows) when the shadow solution is 0 or 2 (cdsp0000's: 2) |
| scene `+0xbc` | cluster `+0x28c4` when the shadow solution (`+0xf9cc`, from `ShadowManager_selectShadowSolution` `0x006ccd10`) is 1 or 3; with 4 it is in no world |

Order with `+0xd4` (map alpha fast) clear, as on this frame:

1. map `+0xac`, OpaqueNoMSAA (MSAA off; Opaque with MSAA), dir 1;
2. characters `+0x7c`, Opaque, dir 1;
3. `PhyreScene_DrawMeshMatrices(1)` (`0x0066b360`): the
   `graphicAddMeshMatrix` list, unculled and unsorted (empty here);
4. `PhyreScene_DrawZPrePass` (`0x00651070`) when asked (nothing here);
5. characters `+0x7c`, Transparent, dir -1;
6. map `+0xac`, TransparentNoMSAA, dir -1;
7. `+0xbc`, Transparent, dir -1, for shadow solutions 1 and 3;
8. `PhyreScene_DrawMeshMatrices(0)` in Transparent.

With `+0xd4` set the transparent order is map, `+0xbc`, z pre-pass,
characters. The opening and close of the frame (the shadow maps, HDAO,
the post-process chain) are outside the scene targets.

**Shadows (corrected in round 2).** Round 1 had cdsp0000 at shadow
solution 0. It is **2**: read in the game at `*g_PhyreScene +0xf9cc`,
and `ShadowManager_selectShadowSolution`'s table agrees
(`docs/formats/scene-maps.md`, Q7). `PhyreScene_RenderShadow`
(`0x00653600`) runs on every field frame, for every solution, before
the scene targets are cleared: for 2, 3 and 4 the cascade pass
`0x006c9e50`, which then writes the cascade matrices and samplers into
the map materials (`LightViewProjection`, `LVP2`, `MySMScaleBias*`;
below), for 0 and 1 `0x006ce6f0`; then, for all but 4, `0x006caee0`.
Both `0x006c9e50` and `0x006caee0` return at once while the `Shadow`
setting (ShadowManager `+0x4060`) is 0, as in the test sandbox, so no
reference capture draws into the shadow targets: on this frame the
1024 x 1024 targets are cleared, nothing is drawn into them, and the
materials still get this frame's cascade values. Cluster `+0x28c4`, the
blob shadows, is in the characters' world for solution 2 (draw 115).

Forcing solution 0 everywhere (a hook on
`ShadowManager_selectShadowSolution` that lets it run, so the level word
`+0x4050` and the cascade constants are still set, and returns 0; and
`PhyreScene_ForceSetShadowSolution` `0x006513f0` storing 0) gives every
map and battle the blob shadow under each character and no shadow maps.
The blob is `Ch_DrawShadowQuad`'s quad with `chry0shadow.fx`, black with
the alpha of the character's tile in the 1024 x 1024 atlas
(`ShadowManager_renderCharacterTiles_006caee0`), which solution 4 maps
(Besaid beach, Mi'ihen, Bikanel) never draw. Open: in maps that were 2-4,
`PhyreScene_fieldMapInitialize` then gives the map materials no cascade
matrices and samplers, so shaders that sample them read whatever is bound
(ffx-demaster `docs/render.md`, `render.shadows`; harness check on Besaid
with `graphicShadowReset_00643e80(1)` after spoofing the level word).

The capture shows exactly that: 128 scene draws (draws with two render
targets and a depth buffer, as `gfx/live.rs` defines scene draws).

| # | Pass (by state) | Family (pixel shader cluster) | Draws |
|---|---|---|---|
| 0-99 | Opaque, map | `phyredefaultlitshader.fx` | 100 |
| 100-114 | Opaque, characters | `phyrechrlitshader.fx` (Tidus `c001`, his `f001` object) | 15 |
| 115 | Transparent, characters world | `abmapshadercone.fx` / `chry0shadow.fx` (blob shadow) | 1 |
| 116 | Transparent, characters | `phyrechrlitshader.fx` (Tidus segment 12) | 1 |
| 117-127 | Transparent, map | `phyredefaultlitshader.fx` 4, `phyrewatershader.fx` 7 | 11 |

The census tells the pass from the state (blend on, or no depth write:
transparent); it agrees with the pass order above.

## Culling

`renderWorld`'s body (`Phyre_PRendering_PRenderer_renderWorldCulledSorted`
`0x009f3c30`) culls and sorts per `PMeshInstanceBounds` before anything is
drawn:

1. `PVisibilityCheck` (`0x009f2580`) takes the camera's view-projection at
   `PCamera +0x70` (depth range -w..w) and the view's z row as the depth
   plane.
2. Jobs of at most 400 bounds (`startJobsForAllBounds` `0x009f2250`) run
   `PCheckVisibleBoundsJob_execute` (`0x009f3160`) on each
   `PMeshInstanceBounds` (`m_min` +0, `m_worldMatrix` +0xc, `m_size`
   +0x10, `m_meshInstance` +0x1c, `m_render_order` +0x20):
   - `m_meshInstance` 0 skips it. That is how the game hides instances:
     `PhyreScene_setMeshVisible` (`0x0066d910`), `SetMeshesVisible`
     (`0x0066e7b0`), the group-id mask (`0x0066d610`) and the 2D layers.
     `setMaterialVisibilityBySetAlpha` does not cull, it writes
     `MaterialColour` = (1, 1, 1, a).
   - The box test is `BoundingBox_IsVisible_00663a30` on `viewproj .
     world`: the eight corners `m_min + {0, m_size}` through the matrix,
     the `ClipOutcode`s (`0x00652470`: bits for x, y, z > w and < -w,
     strict, NaN sets none) ANDed. Culled only when every corner is
     outside one plane.
   - Occlusion (`0x009f4100`) only with occluders; field maps have none.
3. Inside `renderMeshInstance` (`0x005b9a90`) a segment is skipped by its
   FFX hidden byte (segment `+0x14`), a null material ends the instance,
   segments run in four buckets by `m_segmentRenderOrder` (segment +0x1c,
   values 0-3; 4 and up never draw), and in the opaque pass a material
   remapped from Transparent is skipped. The pass picks the technique
   (`selectTechniquePass` `0x005ba690`); no technique, no draw.

Checked on the frame (`census::cull_check`, with the frame's
`scene.ViewProjection` taken back to -w..w by z' = 2z - w): `cdsp00` has
29 instances and 19 draw. Of the 10 that don't, 9 fail the box test with
the World most map instances have (10 x unit); `FBXASC056_0Shape` passes it
and still does not draw: hidden by the game, or its World is not the
common one (open question). No drawn instance fails the test.

## Sort

The same job keys each visible instance:

- centre = `m_size * 0.5 + m_min` (each step in f32), through
  `m_worldMatrix` when set;
- key = `w + (px*cx + py*cy + pz*cz)` of the view's z plane, in f32 step
  by step, the plane negated for dir >= 0: opaque instances sort near to
  far, transparent ones far to near;
- with an entry in `g_DepthBias` (`0x0194299c`, DepthBiasHooks), key =
  `key * scale + bias`.

`std_sort_VisibleEntries` (`0x009f17c0`, MSVC `std::sort`: insertion sort
up to 32 entries, unstable above) orders the entries by
`PCheckVisibleBoundsJob_compareEntries` (`0x009f2d70`): render orders with
`ro / 100000000 == 1` first, then `ro % 1000` ascending, then
`(ro % 100000) / 1000`, then the key. Jobs are merged
(`mergeResults` `0x009f3660`) by the /1e8 class and the key only, the
earlier job first on ties. An instance's segments then draw together, in
segment order within each render-order bucket.

On this frame the map's `m_render_order`s are small integers (0-12) that
put the map's instances in authoring order; the key decides only among
equal orders, e.g. the six water planes (order 4) drawn far to near at
view z -3477, -3229, -1684, -1033, -393, +139. `census::sort_check`
finds the 20 neighbouring instance pairs of the frame in rule order, none
out of order.

## Constants

Every scene draw has one constant buffer per stage (`$Globals`, slot 0).
`allocateAndFillParameterBuffers` (`0x005b5a50`) fills it from up to four
sources: scene-wide values (`PSceneWideParameters_getBuffer` `0x004fdec0`,
re-uploaded only when its key changes), the material's
`PParameterBuffer` copied whole (`0x005b7f30`), per-node values
(`0x005b7fe0`) and streams. A value is written only when its offset is at
least 4 and it fits (`PParameterCaptureBuffer_setValue` `0x004fedc0`).
Semantic ids are the byte at `PShaderParameterDefinition +2`
(`g_ShaderParameterSemanticNames` `0x00b17f40`, `g_SceneWideParameterTable`
`0x00b23bc8`).

Used variables of the frame's 26 shaders (reflection, `ffx_dxbc`), by
source; "uses" counts draws x stages:

| Source | Names | Uses |
|---|---|---|
| derived by PhyreEngine | 13 | 2,752 |
| game state | 26 | 2,595 |
| material parameter, as stored in the cluster | 40 | 707 |
| material parameter, changed at run time | 4 | 4 |

**Derived** (the game supplies only the inputs):

| Variable | Semantic | Value |
|---|---|---|
| `scene.View` | view 3 | `FromMatrix3x4(PCamera +0)`, the inverse of the camera world at scene `+0xfa04` (`graphicCameraSetWorldMatrix` `0x00639860`) |
| `scene.Projection` | projection 2 | `PCamera +0x30` with depth taken to 0..1 (`ConvertProjectionMatrixDepthRangeToZeroOne` `0x00454260`: [10] = f/(n-f), [14] = nf/(n-f)); FOV `+0x154`, aspect `+0x150`, near `+0xb8`, far `+0xbc` |
| `scene.ViewProjection` | viewprojection 6 | Projection . View |
| `scene.ViewInverse`, `scene.EyePosition` | viewi 4, EyePosWS | the camera world matrix, its translation |
| `scene.cameraNearFar`, `cameraNearTimesFar`, `cameraFarMinusNear` | 7-9 | n, f; n f; f - n |
| `scene.ViewportWidthHeight`, `screenWidthHeightInv` | 0xc, 0xf | the viewport; the screen size at `0x00c95100` |
| `WorldView` | modelview 0x83 | View . World |
| `WorldViewProjection` | modelviewproj 0x84 | ViewProjection . World |
| `BoneTransforms` | boneTransforms 0x8f | per `m_skinBones` entry: joint world . inverse bind (`computeJointMatrices` `0x004f6ef0`), the slot after the last bone's first word `0xffffffff` |

The arithmetic, checked bit for bit on all 127 draws that have the
matrices (`census::matrix_checks`). Matrices sit in the buffer row by row
for column vectors (translation in elements 3, 7, 11), and the products
below are in that form:

- `WorldView` = View . World with f32 sums, k = 0 to 3 in order (127/127;
  the same in f64 gives 103/127);
- `WorldViewProjection` = ViewProjection . World with each sum in f64 and
  one rounding to f32, the x87 at 53 bits (127/127; with f32 sums 103/127,
  up to 6 ULP off; WorldView . Projection is never exact);
- `ViewProjection` = Projection . View (127/127).

**Game state** (written by FFX's code; the renderer copies it):

| Variable | Where it comes from |
|---|---|
| `World` (model 0x82) | the instance's world matrix as the game sets it: map instances 10 x unit (PS2 units), moving map parts through `PhyreScene_setObjectRotation`/`Translation` (`0x0066db00`/`0x0066e3e0`; one deck instance turned about y), characters through `setMeshInstanceWorldMatrix` (`0x0066d810`). Never the file's node world (identity for this map) |
| `scene.GlobalAmbientColor`, `FogColor`, `FogNear`, `FogFar`, `FogCurve`, `FogLimit`, `BGColor`, `GlobalTime`, `ViewProjection2` | `g_PhyreApplication` (`0x00cc9cd8`) `+0x50`, `+0x60`, `+0x70`, `+0x74`, `+0x78`, `+0x7c`, `+0x80`, `+0x120`, `+0x90` (`docs/formats/render.md`, "Scene state"; fog through `FogEffectManager` `0x006e36f0`) |
| `CharacterAmbientLight`, `CharacterShadeColor`, `CharacterShadeCoeff`, `Light0-2` | character lighting, written into the character materials (`graphicCharacterLighting*`, `...SetLightMatrix`) |
| `LgtMatVec`, `LgtMatClr` | PS2 light matrices in map materials (`graphicPS2LightingSetupEffect`) |
| `LightViewProjection`, `LVP2`, `MySMScaleBias`, `MySMScaleBias2` | shadow cascades, written into material buffers by `PhyreScene_RenderShadow` |
| `ClothWorld` | the cloth simulation (`Ch_Cloth*`), five Tidus draws |

**Material parameters** (`PParameterBuffer`, `docs/formats/phyre.md`):
`MaterialColour`, `fgEff`, `clrScaleFactor`, `AlphaThreshold`,
`EnvironmentFactor`, `EnvironmentMapRotation`, `GrayScaleParam`,
`MultiTexBlendFactor`, the water's `ScrollSpeed_*`, `SpecularColor`,
`SpecularShininess`, the `VertexColor*` set, the texture-animation
`AnimScale_MAYA*`/`AnimTranslate_MAYA*`/`layer1-6`: every captured value is
the stored one on this frame. The game does write some of them
(`MaterialColour` alpha for fades, texture animation, `GradualFade_*`), so
the builder must read them from the live material, not the file. The
`SgSpec_*` set (one draw) differs from the file: written at run time.

## Asset parity

Keys are `ffx-drawdiff`'s (`docs/drawdiff.md`, `crates/ffx-drawdiff/src/key.rs`):
shaders by their bytecode, textures by description plus each slice's top
level, static buffers by width, bind flags and contents. The asset side
(`ffx_scene::assets`) makes the same objects from the clusters:

| Object | What the game makes from the cluster |
|---|---|
| shader | each DXBC blob of `shaders/d3d11/<effect>.fx#<md5>.phyre`; the effect name is the shader's family |
| vertex buffer | one per `PDataBlock`: `stride * count` bytes from `m_offsetInVertexBuffer`, `D3D11_USAGE_IMMUTABLE`, bind `VERTEX_BUFFER | SHADER_RESOURCE` (9); the draw binds the segment's blocks in a shader-dependent slot order, offset 0 |
| index buffer | one per segment: the `u16` indices from `m_offsetInIndexBuffer`, immutable, bind `INDEX_BUFFER | SHADER_RESOURCE` (10), `R16_UINT`, start 0, base 0 |
| texture | the file's mip chain without its first 2 levels on `TQ_LOW` (all 96 file textures of the frame, 64x512 to 4096x4096: a 2048 file becomes a 512 texture with 10 levels); DXT1/3/5 as BC1/2/3_UNORM (71/74/77); rows as stored (bottom first, no flip); cube maps with 6 slices, each face's chain in turn in the file |
| input layout | one element per shader input from the segment's streams (table below) |

| Stream (`m_type`) | Element (DXGI format) |
|---|---|
| `Vertex`, `SkinnableVertex` (float3) | `POSITION0` (6) |
| `Normal`, `SkinnableNormal` (float3) | `NORMAL0` (6) |
| `Color` (float4) | `COLOR0` (2) |
| `ST` (float2) | `TEXCOORD0`, and `TEXCOORD4/6/7` for the water (16) |
| `Tangent` (float3) | `TANGENT0` (6) |
| `SkinWeights` (float4) | `BLENDWEIGHTS0` (2) |
| `SkinIndices` (four bytes) | `BLENDINDICES0` (30, `R8G8B8A8_UINT`) |

A shader input the segment has no stream for reads a stand-in: four map
draws (`phyredefaultlitshader` with `TEXCOORD1` as float3) bind the `ST`
block a second time in that slot.

Results on `fw1-a` (bindings, distinct objects):

| Object | Matched | Dynamic | Render target | Unexplained |
|---|---|---|---|---|
| shaders | 256 (26) | | | 0 |
| vertex buffers | 550 (527) | 9 (1) | | 0 |
| index buffers | 127 (120) | 1 (1) | | 0 |
| textures | 155 (96) | | 27 (2) | 0 |
| input layouts | 122 (19) | 6 (4) | | 0 |

- Dynamic, with evidence (`D3D11_USAGE_DYNAMIC`, the 12 MB vertex ring and
  1.5 MB index ring): the blob shadow under Tidus (draw 115,
  `abmapshadercone`/`chry0shadow`, CPU-built quad), and a sixth vertex
  stream (stride 16) on five Tidus draws, the cloth simulation's
  positions (they also read `ClothWorld`): CPU-built.
- Render targets (bind flags with `RENDER_TARGET`): the two 1024 x 1024
  `MySMSampler` shadow maps (and an 8 x 8 one), sampled by 10 map draws;
  nothing is drawn into them (`Shadow=0`: `PhyreScene_RenderShadow`'s
  passes return at once, "Passes" above), so they keep what they held
  since they were made. The builder binds them as the host's
  (`External::MaterialTexture`).
- Texture levels (round 2 wire). A texture's device levels are not
  always the file's chain: `Phyre_PBinary_loadClusterTextures` reads the
  dropped top levels into the start of its buffer, and the upload
  (`0x0058fc20`) halves width and height per level without a floor and
  takes a side that reaches 0 as the top level's, so a non-square
  texture's last levels sit past the levels read and hold bytes of a
  dropped level (cdsp0000: four 16x128, 32x128 and 128x32 textures).
  Full captures key a texture by its top level and don't see it; hash
  captures hash every level. `ffx_scene::build` does the same
  (`upload_subresources`).
- Characters: Tidus's model and texture match only the mod's files.
  Without `FFX_SCENE_MODS` the census lists his static buffers as
  "skinned" (not in the archive), and his 8 layouts and 13 texture
  bindings as unexplained.

## Scene inputs

Round 2 (R2a). While a capture runs, `ffx-hook` reads what the builder
needs at every map `renderWorld` and writes it into `capture.ffxgfx` as
`SceneInputs` messages (tag 15, `ffx_gfx::scene_inputs`). Each frame's
messages come right before its `Frame`. `ffx_scene::inputs` reads them
and derives the matrices.

- **Where.** The hook wraps
  `Phyre_PRendering_PRenderer_renderWorldCulledSorted` (`0x009f3c30`,
  thiscall(renderer, world, camera, context, dir), the body of
  `PApplication_renderWorld`) for world scene `+0xac`. Another agent may
  hook `renderWorld` itself, so this hook sits one call below it. With no
  capture running it costs one atomic load per call.
- **When.** `renderWorld` runs on the **main thread**, right after the
  update, and records PhyreEngine's commands. The render thread makes the
  Direct3D calls a frame later. Each record is therefore tagged with the
  harness's update label (`update`, the label the frame's flip gets). At
  each captured `Present` the capture gives the records of the update that
  frame shows (`harness::present_label`) its frame number, and leaves out
  records no captured frame shows. Four calls per update: `OpaqueNoMSAA`
  (dir 1), `TransparentNoMSAA` (-1), `postOpaque` (1) and
  `postTransparent` (-1). All four see the same camera and Worlds.
- **What** (no pointers: objects become indices):
  - pass, dir, world offset;
  - the camera's first `0x180` bytes. This pointer is 4 bytes past scene
    `+0xdc`'s: view 3x4 `+0`, projection `+0x30`, culling
    view-projection `+0x70`, near `+0xb4`, far `+0xb8`, aspect `+0x14c`,
    FOV `+0x150`;
  - `g_PhyreApplication +0x40..+0x180` (ambient, fog, background,
    `ViewProjection2`, `GlobalTime`) and the screen size `0x00c95100`;
  - the group mask (`+0xfb14`), shadow solution (`+0xf9cc`), alpha-fast
    flag (`+0xd4`) and 2D layers (`+0x1140`) of the scene;
  - `g_DepthBias` entries (keys resolved to bounds);
  - every `PMeshInstanceBounds` in the visibility check's walk order: the
    world's class table, then each instance list's live objects, free
    slots passed (`PWorldCursor_*`, `ffx.sym`). Each carries `m_min`,
    `m_size`, the live `m_render_order`, `*m_worldMatrix`, and the
    instance as (cluster, slot in the cluster's `PMeshInstance` list,
    which is the file's instance order);
  - each instance's world 3x4, segments (material index, joint, hidden
    byte `+0x14`, id, `m_segmentRenderOrder`) and material set. Also
    FFX's `m_groupID` `+0x48`, `m_DObjKind` `+0x4c`, `m_dObjFlag` `+0x54`
    and object id `+0x58`;
  - each material's remapping (`+8`, `+0xc`) and its live
    `PParameterBuffer` (size word included, so a parameter's offset indexes
    it);
  - the clusters: position in the world's list, the model's symbol (scene
    `+0xd8`, `cdsp00`), and their instance lists.
- **The `Wire` block** (round 2 m4-wire, `ffx_gfx::scene_inputs::Wire`):
  an extension block after the record and its character part's flag (tag `0x57520001`;
  version 1 readers stop before it, `read_extensions` skips tags it does
  not know). It holds the camera's world matrix (`*(PCamera +0xb0)`, the
  source of `EyePosition`), the texture quality
  (`Phyre_PRenderInterface +0x1340`, `0x00c96240`) and the anisotropy
  setting (`0x00c25370`), each cluster's file (its first
  `PAssetReference`'s id up to the `#`), and each material's texture
  slots: per `0x42`/`0x44` definition of its effect variant, whether the
  slot holds a texture (`+0xc` of the 16-byte slot) and that texture's
  asset id, found in the asset reference tree (`0x00c9424c`, walked as
  `Phyre_PAssetReference_GetAssetReferenceByAsset` `0x0045cb50` does).
  Texture animation shows there: on `bsil0000` material 159 holds
  `tex/texswap/12032_20_000_000_4_128_128_001.dds`, not its cluster's
  texture. Run-time textures (the shadow maps) have no asset id.
- **Check.** `ffx-scene`'s `tests/inputs.rs` (`FFX_SCENE_INPUTS`, with
  `FFX_GAME_DIR`) ties every map draw to its instance through the census.
  On fw-101 and fw-459 of two original runs (`~/ffx-work/m4/captures/
  r2a-{a,b}`) it reproduces `scene.View`, `scene.Projection`,
  `scene.ViewProjection` (536/536), `World`, `WorldView` and
  `WorldViewProjection` (446/446) bit for bit. The two runs' records
  differ only in `GlobalTime` (`+0x120`, by about 1e-6).

## Round 2: open questions

1. Texture quality: which function drops the levels, and how many
   `TQ_MEDIUM`/`TQ_HIGH` drop (the `TextureQuality` key at `0x00b7c870`,
   the settings at `0x00c8a248..`). The builder must read the setting.
2. The mods: should reference captures run without the user's mods
   (harness option), or should the builder read the mod folder as the hook
   does? Either way the scene builder needs the hook's file resolution.
3. `FBXASC056_0Shape` (cdsp00) passes the box test with the common World
   and does not draw: find what hides it (`m_meshInstance` 0, a 2D layer,
   group ids) and record the game state that does it.

   **Answer (R2a): nothing hides it; it is outside the frustum.** Its World
   is not the common one. On fw-101 and fw-459 (two runs each,
   `ffx-scene/tests/inputs.rs`), slots 27 `FBXASC056_0Shape` and 28
   `FBXASC057_0Shape` hold the same mesh, mirrored. Both are turned about
   y (3x4 words 3 and 4: -2.07, -9.78, so cosine -0.21) and moved to
   x = +123.7 and -123.7 (words 8-10). With its own World, slot 27 fails
   `BoundingBox_IsVisible_00663a30` on both frames. Nothing else holds it
   back: its bounds' `m_meshInstance` is set, no segment is hidden, and
   its `m_groupID` 0xc is inside the group mask (`PhyreScene +0xfb14` =
   0xfffffff7; the mask clears only group 3, which no cdsp00 instance
   uses).

   Round 1 got this wrong in two ways. It tested every undrawn instance
   with the common World. And the census ties a draw to an instance through
   its vertex data. Two separate meshes with equal contents look like one,
   so draw 99, which is `FBXASC057_0Shape` (World x = -123.7), was
   credited to `FBXASC056_0Shape`, the first of the two. With the
   snapshot's Worlds every map instance that drew passes the box test, and
   every map instance that did not draw fails it (10 on fw-101, 9 on
   fw-459). The builder must cull with the live World (scene inputs,
   below), not the file's or the common one.
4. World matrices: map instances get 10 x unit and some a rotation at run
   time; trace who writes them for every map instance (the object tree at
   scene `+0x1198`).

   **Answer (R2a).** Each map instance has one world matrix: the 3x4 at
   `*(PMeshInstance +4)`. Its bounds' `m_worldMatrix` (`+0xc`) points at
   the same matrix (`world_shared` in the snapshot: true for all 29). Two
   writers fill it:
   - **At load**, `PhyreScene_doOnFieldMapAssetLoaded` (`0x0065af60`),
     for the model cluster (`AssetListReader_isModel`), walks every
     `PMeshInstance`. It takes the file's node world
     (`PMatrix4_FromMatrix3x4` `0x00451080`) times 10 x unit for a 3D map
     (`(-2.5, -2.5, 1)` scale and a `(-256, -224)` shift for 2D maps),
     converts it back to a 3x4 (`PMatrix4_toWorldMatrix3x4`), copies it
     into `*(inst +4)` and points `*(inst +0x1c) +0xc` at it. The file's
     node worlds are identity on cdsp00, which gives the common
     `[0, 10, 0, 10, 0, 0, 10, 0, 0, 0, 0, 0]`.
   - **Every frame, for the map's objects (DObjs)**, the main thread runs
     `yiDisp` (`0x00908480`), then `yiRender_draw` (`0x0090bf10`, ported
     in `ffx_field::yi`), then `graphicObjectSetTranslation` /
     `graphicObjectSetRotation` (`0x006426c0` / `0x006426a0`). These reach
     `PhyreScene_setObjectTranslation` (`0x0066e3e0`) and
     `PhyreScene_setObjectRotation` (`0x0066db00`). Both find the object
     id in the object tree (scene `+0x1198`) and rewrite each listed
     mesh's 3x4 (`docs/formats/render.md`, "Object translation/rotation").
     Event scripts (`yiSetRotTransDObj` `0x0090a750`,
     `yiSetRotTransGroup` `0x0090a830`, the `yiAdd*` pair) call the same
     two.

   A write watch confirmed it (`watch-worlds` in `FFX_GFX`,
   `ffx-hook` `world::watch`: x86 debug registers on four matrix words of
   every thread, armed at a capture's first map `renderWorld`; run
   `r2a-w` of `draw-field.txt`). Over the two updates after arming:

   | Instance | Writes per update | Writer |
   |---|---|---|
   | slots 0 and 1 (DObj kind 1, `FBXASC0491/0492`, swaying) | 1 | `setObjectRotation` (store before `0x0066e369`) |
   | slot 22 (object id 3: the winch wheel `FBXASC051_0Shape`, not the crane; corrected below) | 2 | `setObjectRotation` (`0x0066e357`) and `setObjectTranslation` (`0x0066e47c`) |
   | slot 2 (a common one) | 0 | none |
   | Besaid slots 7-9 | 2 | both (`0x0066e34e`, `0x0066e476`) |

   **Correction (ffx-remix M3, `docs/formats/scene-maps.md`, "Map entry
   state", "Script placements at map entry").** Object id 3 on `cdsp00`
   is the salvage ship's winch wheel, `FBXASC051_0Shape` (`m_objID` 3,
   `m_groupID` 13), not a crane; the crane arm is `FBXASC057_0Shape`
   (worker 23). Event script worker 25 binds the wheel's group, places
   it at map entry and keeps turning it about x in its loop, which fits
   the two writers on it every update.

   Every write came from the main thread. So the Worlds are game state that
   moves every frame for the DObjs and is fixed at load for everything
   else. The builder takes them from the snapshot. `World` = the 3x4
   through `PMatrix4_FromMatrix3x4`, transposed into the shader's order
   (`ffx_scene::inputs::world`). This is bit-exact on all 446 map draws of
   the four captures.
5. The scene-wide buffer's re-upload (only when its key changes) and the
   per-draw constant buffer versions: the D3D call stream depends on them.

   **Answer (R2a).** The scene-wide values are not a Direct3D buffer of
   their own:
   - `PSceneWideParameters_getBuffer` (`0x004fdec0`) keeps them in a CPU
     parameter capture buffer, in a ring of slots (count at `+0x10`).
     `allocateAndFillParameterBuffers` (`0x005b5a50`) calls it for every
     draw.
   - It first builds a `PSceneWideParametersKey` (`0x004fe940`): the
     camera's view (`+0..+0x2c`), its projection (`+0x30`, depth taken to
     0..1 with near `+0xb4` and far `+0xb8`), the camera's `+0xb0`, and
     the viewport. It compares the key with the cached one that
     `+0x34 & 1` picks. When they are equal it reuses the slot. When they
     differ it moves to the next slot, waits for that slot's refcount to
     reach 0 (printing "Waiting for Scene Wide parameter buffer to become
     free" after 10⁶ sleeps), and refills it. So on the map passes it is
     refilled once per camera change, and the copies stay the same within
     a pass.
   - `allocateAndFillParameterBuffers` copies those values into each
     draw's own `$Globals`, together with the material's
     `PParameterBuffer` and the per-node values.
   - The draw command (`0x005ba380`) refcounts the capture buffers into
     0x48-byte entries of the render ring
     (`Phyre_PRendering_PRenderRingBufferSegmented_allocate`).

   All of this runs on the **main thread**: `renderWorld` records
   PhyreEngine commands one update ahead of the render thread, which makes
   the Direct3D calls (see "Scene inputs" below). On the Direct3D side every
   draw writes the `$Globals` buffer of each stage it binds (vs slot 0, ps
   slot 0) exactly once. In `fw1-a`, 197 draws over 78 buffers, each
   buffer's version rises by one per draw that binds it: no version is
   reused and none is skipped (`~/ffx-work/m4/r2a/cbver.py` over
   `frame.txt`). Each shader program has its own buffer, so versions are
   per program and stage, and the starting values carry over from earlier
   frames. A builder that issues one constant-buffer write per draw and
   stage reproduces the call stream. Version numbers differ by object
   lifetime, and drawdiff compares contents, not versions.
6. `m_render_order` across maps: values seen are 0-12; other maps may use
   the /1e8 class or the thousands field, and lists over 400 bounds go
   through the merge comparator, which ignores the `% 1000` fields.
7. The shadow maps: ~~solution 0 here~~ solution 2 here (corrected in
   round 2, "Passes" above); `PhyreScene_RenderShadow` and the
   ground-shadow cluster `+0x28c4` for the other solutions.

   **Answer (R2d, `docs/formats/scene-maps.md`, Q7).** Solutions 0, 2 and
   4 are measured (`kami0100`, `cdsp0000` and `bsil0300`, `bsil0000`);
   `PhyreScene_RenderShadow` runs for all of them and, with `Shadow=0`,
   draws nothing.
8. Unidentified: `0x006ada20` and `0x00695ba0` in `PhyreScene_render`,
   `0x005bafc0`; whether the multithreaded branch of
   `renderMeshInstanceList` ever runs.
