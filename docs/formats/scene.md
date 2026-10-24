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
| scene `+0x7c` | the characters (cluster `+0x11e4`, moved there by `attachChrClusterTo3DWorld`), and cluster `+0x28c4` (ground shadows) when the shadow solution is 0 or 2 |
| scene `+0xbc` | cluster `+0x28c4` when the shadow solution (`+0xf9cc`, from `ShadowManager_selectShadowSolution` `0x006ccd10`) is 1 or 3 |

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
characters. The opening and close of the frame (shadow maps
`PhyreScene_RenderShadow` `0x00653600` for solutions 2-4, HDAO, the
post-process chain) are outside the scene targets; on this frame the
shadow-map target is cleared and nothing is drawn into it.

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
  nothing is drawn into them this frame, so their contents date from an
  earlier frame (round 2 must reproduce them).
- Characters: Tidus's model and texture match only the mod's files.
  Without `FFX_SCENE_MODS` the census lists his static buffers as
  "skinned" (not in the archive), and his 8 layouts and 13 texture
  bindings as unexplained.

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
4. World matrices: map instances get 10 x unit and some a rotation at run
   time; trace who writes them for every map instance (the object tree at
   scene `+0x1198`).
5. The scene-wide buffer's re-upload (only when its key changes) and the
   per-draw constant buffer versions: the D3D call stream depends on them.
6. `m_render_order` across maps: values seen are 0-12; other maps may use
   the /1e8 class or the thousands field, and lists over 400 bounds go
   through the merge comparator, which ignores the `% 1000` fields.
7. The shadow maps: solution 0 here; `PhyreScene_RenderShadow` and the
   ground-shadow cluster `+0x28c4` for the other solutions.
8. Unidentified: `0x006ada20` and `0x00695ba0` in `PhyreScene_render`,
   `0x005bafc0`; whether the multithreaded branch of
   `renderMeshInstanceList` ever runs.
