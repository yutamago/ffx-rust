# The scene builder: the map world in Rust (M4 round 2)

`ffx_scene::build::build(clusters, inputs)` makes the draws of the map
world (scene `+0xac`) for one frame: the opaque pass, then the transparent
pass, as `ffx-draw` `DrawState`s and `DrawCall`s, with every object they
use. Its spec is the round 1 census (`docs/formats/scene.md`) and the
round 2 map census (`docs/formats/scene-maps.md`). Crate:
[`crates/ffx-scene`](../crates/ffx-scene).

On cdsp0000 its 111 draws are the captured map draws (scene draws 0-99 and
117-127) of every reference capture, through `ffx-drawdiff`, apart from one
allowed field (below).

```bash
# the gate (skips without FFX_GAME_DIR and the captures)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
FFX_SCENE_CAPTURES=$HOME/ffx-work/m4/captures/fw1-a/capture.ffxgfx:$HOME/ffx-work/m4/captures/fw3-b/capture.ffxgfx \
  cargo test --release -p ffx-scene --test builder
# one capture: build, diff, report (and the rebuilt capture, for drawdiff)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-scene --example builddiff -- \
  ~/ffx-work/m4/captures/fw1-a/capture.ffxgfx [out.ffxgfx] --allow crates/ffx-scene/tests/builder-allow.txt
```

## API

| Item | What it is |
|---|---|
| `build::Clusters::load(read, model_path)` | The static data: the map model (`map::MapModel`) and every effect variant (`effect::EffectVariant`) and texture its materials import. `read` maps an archive path to bytes (the archive, or the mod folder first) |
| `frame::SceneInputs` | The game state of the frame (the trait the live snapshot will implement): settings, camera, pass names, the bounds' order, shown flags, worlds and render orders, segment hidden bytes, depth biases, material buffers, run-time material textures, the scene-wide buffer, viewport and scissor |
| `build::build(&clusters, &inputs) -> Result<Built>` | `Built { resources, draws, notes }`: `draws` are `MapDraw { pass, instance, segment, material, state, call }`, opaque first; `notes` name what was not done exactly (per-node semantics left 0, unknown entries) |
| `build::Resources` | The objects, indexed by the handles in the states: shaders, layouts, blend/depth/raster/sampler descriptions, buffers (static vertex and index buffers, one dynamic constant buffer per draw and stage), textures with every subresource, views. A view is a texture's, or `External` (the scene targets, textures the game sets at run time) |
| `upload::Uploaded::new(&mut backend, &resources, host_view)` | Makes the objects on an `ffx-draw` backend; `up.state(&draw.state)` moves a state onto the backend's handles. What the in-game path (R2c) issues |
| `capture_inputs::CaptureInputs::new(&capture, &clusters, settings, msaa)` | Round 2's inputs, read back from a capture (below) |
| `emit::replace_map_draws(...)` | A copy of a captured frame with its map draws swapped for the builder's, for `ffx-drawdiff` |

Dev tools (examples): `builddiff` (above), `clusters` (a cluster's
namespace, lists, objects, strings), `effect` (an effect variant's passes,
states, programs and location tables), `material` (a model's materials,
their sampler states and textures), `instances` (a model's instances as
the builder reads them), `drawinfo` (what a captured scene draw binds).

## What the builder does

Per pass, as `PApplication_renderWorld` (`0x0062be70`) and
`renderWorldCulledSorted` (`0x009f3c30`) do.

### Culling and order (`cull.rs`, `sort.rs`)

- The bounds go to jobs of 400 in the world's order. Each job skips
  bounds whose `m_meshInstance` is 0 (`SceneInputs::instance_shown`),
  culls with `BoundingBox_IsVisible_00663a30` on the camera's `+0x70`
  matrix times the world (`PMatrix4_FromMatrix3x4`, `PMatrix4_Multiply`),
  and keys the survivors (`PCheckVisibleBoundsJob_execute` `0x009f3160`).
- The key is the bounds' centre (`size * 0.5 + min`, each step stored),
  through the world, dotted with the view's z row (`PVisibilityCheck`
  `0x009f2580`: elements 2, 6, 10, 14 of `FromMatrix3x4(PCamera +0)`,
  negated for the opaque pass): `s = px*cx`, `s += py*cy`, `s += pz*cz`,
  `key = s + w`, each sum stored as a float, the products exact (x87 at
  53 bits). A `g_DepthBias` entry makes it `key * scale + bias`, stored
  once.
- Each job sorts with MSVC's `std::sort` as FFX.exe instanced it
  (`0x009f17c0`): insertion sort up to 32 entries (`0x009f0d20`), above
  that the fat partition around a median of three, or of nine above 40
  (`0x009f1c60`, `0x009f1230`), with heap sort after 1.5 log2(n)
  divisions (`0x009f0f50`, `0x009f1970`, `0x009f0b60`). The comparator is
  `compareEntries` (`0x009f2d70`). Jobs merge by the `/1e8` class and the
  key only, the earlier job first on ties (`mergeResults` `0x009f3660`).

### Per instance (`renderMeshInstance` `0x005b9a90`)

- The segments run in four buckets by `m_segmentRenderOrder` (0-3), in
  mesh order within a bucket.
- A segment whose material slot is null ends the instance.
- In the pass named `Opaque` (the static name, not `OpaqueNoMSAA`), a
  material remapped from `Transparent` to anything else is skipped.
- The material's remapping applies (`m_remapFrom == pass`: use
  `m_remapTo`), then the effect variant's render pass of that type
  (`selectTechniquePass` `0x005ba690`, `0x004e71d0`). No render pass, no
  draw.
- The shader is the render pass's first. The game searches its context
  variants for the node (`0x004ef440`, the effect's `NUM_LIGHTS` switch);
  that search is not ported, and every cdsp0000 map draw uses the first.
- A segment with the FFX hidden byte (`+0x14`) set is not drawn.
- One draw per shader pass.

### Per draw

| Part | From |
|---|---|
| Programs | The shader pass's vertex and fragment programs (`PShaderProgramD3D11.m_compiledCode`) |
| States | The pass's `PShaderPassStateD3D11`: its `D3D11_RASTERIZER_DESC`, `D3D11_DEPTH_STENCIL_DESC` and `D3D11_BLEND_DESC` as stored, made as they are (`0x0058d4e0`) and bound as they are (`0x0058c6c0`: blend factor 0, sample mask `0xffffffff`, the pass's stencil reference). The opaque passes' states have blending off and the variants' own culling; nothing is changed at run time |
| Input layout | One element per `PStreamInputDescD3D11` of the vertex program: its semantic, index, format and slot, at the stream's offset in its block |
| Vertex buffers | Per input, one of the segment's streams of its render type, in block order: when the instance's segment context binds the type and semantic index (`PMeshInstanceSegmentStreamBinding`: the water's `TEXCOORD4/6/7`), the stream whose rank is the binding's `m_inputSet`'s rank among the sets the context binds for that type (cdsp00 numbers them 1, 2, bsil00 0, 1; a rule read from the data, not from the code), else the first. That block's buffer at offset 0 with its stride. This covers round 1's stand-in: `TEXCOORD1` reads the first `ST` stream |
| Index buffer | The segment's `u16` indices, `R16_UINT`, offset 0; the call is `DrawIndexed(count, 0, 0)` on a triangle list (`m_primitiveType` 2) |
| Static buffers | One per `PDataBlock` and one per segment's indices: `IMMUTABLE`, bind 9 or 10 (`docs/formats/scene.md`, "Asset parity") |
| Textures | The material's texture in the sampler slot, its chain from the level `Phyre_PBinary_loadClusterTextures` (`0x00465ab0`) starts at (`assets::texture_start_level`: `TQ_HIGH` 0, `TQ_MEDIUM` 1, `TQ_LOW` 2 top levels, none from a texture with that few levels past the first, fewer while the new top would be under 4 px); `BC1/2/3`; cube maps as 6 slices; a `TEXTURE2D` or `TEXTURECUBE` view of every level. A slot the cluster leaves empty binds the game's run-time texture when it set one (the shadow maps: `External::MaterialTexture`), else nothing (cdsp0000's `EnvironmentMapSampler`) |
| Samplers | The material's `PSamplerState` in the slot, described as `PSamplerStateD3D11` (`0x00595bf0`) does: filter from the min and mag filters and the compare flag `m_flags` bit 6 (`0x0058aa10`), `0x55` when the anisotropy setting is above 1; wrap 1 wrap, 3 border, 4 mirror, else clamp (`0x0058ab10`); the LOD bias; `m_maxAnisotropy` truncated; `LESS_EQUAL` with the compare flag, else `NEVER`; border bytes times the `f64` 1/255 at `0x00b34258`; the LOD range of `CD3D11_SAMPLER_DESC(D3D11_DEFAULT)` |
| Texture and sampler registers | The pass's texture location tables: an entry of the material group names a tweakable by its offset; a sampler with a texture (`0x42`, `0x44` for a cube) goes to `t<register>`, a sampler state (`0x47`) to `s<register>` |
| Targets, viewport | The scene's colour and normal targets and depth buffer (`External`); the viewport and scissor of the inputs |

### Constant buffers

Every program has one `$Globals` buffer (`m_globalConstantBufferIndex`,
`m_constantBufferSize`). The pass's parameter location tables (vertex and
fragment) list, in three groups, where each value comes from and where it
goes: entries `[s0, s0+c0)` read the scene-wide buffer, `[s1, s1+c1)` the
material's `PParameterBuffer` (values at their definitions' offsets), and
`[s2, s2+c2)` the per-node buffer. Each entry copies `size` bytes from its
offset to its target in the constant buffer (`allocateAndFillParameterBuffers`
`0x005b5a50`). The builder does the same, starting from a buffer of
`0xcd` bytes: the game's heap fills new blocks with `0xcd`
(`UserHeap_AlignedMalloc` `0x006fb770`), and the words no entry writes keep
it. The shaders read such words: `scene.GlobalAmbientColor.w` is
`0xcdcdcdcd` in every captured map draw.

The per-node buffer is the variant's untweakable definitions:

| Semantic | Value |
|---|---|
| 0x82 `World` | the instance's world (cb layout) |
| 0x83 `WorldView` | `View . World`, f32 sums, k = 0 to 3 |
| 0x84 `WorldViewProjection` | `ViewProjection . World`, f64 sums rounded once (the camera's `ViewProjection`, not a product of the builder's) |
| 0x85 `WorldInverse`, 0x86 `WorldViewInverse` | left 0 and noted; no map shader of cdsp0000 reads them |

## Inputs from a capture (`capture_inputs.rs`)

Until the live snapshot (R2a, `inputs.rs`) is merged, the inputs come from
the capture through the same location tables, inverted:

- each captured scene draw whose index and vertex buffers are a map
  segment's is a map draw; its shader pass is the one whose programs it
  bound; its instance is the one using the segment (with a shared mesh,
  the one already seen with its `World`, else the first not seen);
- the scene-wide buffer from the scene-wide entries;
- each drawn instance's `World` from the per-node entries; an instance
  nothing draws gets the `World` most drawn instances have;
- material values the game writes at run time (`LightViewProjection`,
  `LVP2`, `MySMScaleBias`, `MySMScaleBias2`: the shadow cascades of
  `PhyreScene_RenderShadow` `0x00653600`) from the material entries. All
  other material values are the cluster's, so the gate checks them;
- the camera's `View` and `ViewProjection` from `scene.View` and
  `scene.ViewProjection`; the culling matrix by taking the latter's depth
  back to -w..w (`z' = 2z - w`);
- an instance that passes the box test and draws nothing is taken as
  hidden by the game (cdsp0000: `FBXASC056_0Shape`, `m_groupID` 0xc);
- render orders are the file's, plus 100,000,000 for `m_DObjKind` 1
  (`MorphManager_registerClusterMorphs` `0x006aca10`; cdsp00's
  `FBXASC0491_0/_1`, `0492_Shape0`, `0493_0/_1`);
- run-time material textures are the captured views in their registers;
  hidden bytes and depth biases are taken as unset;
- settings from the test sandbox's `GameSetting.ini` (`TQ_LOW`,
  `Anisotropic=1`); MSAA off (pass names `OpaqueNoMSAA`,
  `TransparentNoMSAA`).

The captures fw1..fw3 were taken with the user's mods. No mod replaces a
map, texture or shader file (they are character models and one
`item.bin`), so the map draws are the unmodded game's.

## Results

| Capture | Map draws | drawdiff (scene part) |
|---|---|---|
| fw1-a, fw1-b, fw2-a, fw2-b, fw3-a, fw3-b (round 1, user's mods) | 111 | 134 items, 0 differences, 7 allowed |
| cdsp0000-a, cdsp0000-b (round 2 harness, no mods) | 111 | 134 items, 0 differences, 7 allowed |

Building takes about 2 ms; loading the clusters about 80 ms.

### Allowed differences (`crates/ffx-scene/tests/builder-allow.txt`)

| Field | Draws | Cause |
|---|---|---|
| `ps.sampler2.max_anisotropy` (`phyrewatershader`, `CubeMapSamplerState`) | the 7 water draws | The material's sampler state has `m_maxAnisotropy` 0.0, so the game and the builder pass `MaxAnisotropy` 0. The capture has no sampler object with that description: the device returned the existing object 0x3a311fa0, recorded with `MaxAnisotropy` 1 and otherwise equal (filter 0x15, clamp, `NEVER`). Direct3D 11 hands back an existing state object for an equivalent description, `MaxAnisotropy` does not matter to a non-anisotropic filter, and the graphics layer records an object's description from the last create that returned it (`ffx-hook` `gfx/mod.rs` `create_state`). In-game the builder's call gets the same object (R2c to confirm) |

`SgSpec_CameraVec[0].z`, the junk read named in the round plan, is not in
the map world: its one draw is a character draw (scene draw 112,
`phyrechrlitshader`, the `Ch_DrawMain` / `graphicSgSetSpecular` path), so
the builder's gate has no entry for it.

## Other maps (bonus, not gated)

`builddiff` takes the model from the capture's scene name (`bsil0000`:
`map/bsil/bsil00/`), one cluster.

| Capture | Matched map draws | Result |
|---|---|---|
| `bsil0000-a` | 79 | 140 of 143 scene items equal; 11 moved, 1 missing and 1 extra: the map world holds draws the builder does not make (13 map draws are not matched: other clusters or CPU-written vertices), which changes the order. Changed: the water's `max_anisotropy` (the cdsp0000 allow entry's cause again, sampler 1) and one `phyredefaultlitshader` draw whose `UVMorphWeight` (0.0495 in the game) and texture (another 32x32 BC3) are set at run time (UV morphing, texture animation) |
| `bsil0300-a` | 43 | 99 of 100 equal; 17 moved, 1 extra. Many map draws bind the 12 MB dynamic vertex ring for `POSITION` (morphing map parts, `MorphManager`), so neither the inputs nor the builder see them |
| `kami0100-a` | - | The map is a 2D-layer world (`scene0layer*`); no `kami01.dae.phyre` |

## Open items

1. **Multi-cluster worlds.** World `+0xac` can hold several clusters (the
   2D layers, other models); the builder takes one model. The bounds list
   and its order across clusters come from the snapshot.
2. **CPU-written map vertices** (bsil03's morphing parts: `POSITION` from
   the dynamic ring). Needs the morph code (`MorphManager`) or its output
   from the snapshot.
3. **Segment stream bindings with several entries** are read as `count`
   objects in a row; the pointer-array fixups that place them are not
   decoded (`ffx-phyre` doesn't unpack them). The `m_inputSet` rank rule
   is read from cdsp00 and bsil00, not from the code that binds the
   streams.
4. **The context variant search** (`0x004ef440`) is not ported: the first
   shader of the render pass is taken.
5. **Per-node semantics** other than `World`, `WorldView` and
   `WorldViewProjection` are left 0 (noted per build).
6. **The x87 arithmetic of the culling matrix product and the point
   transform** is taken as 53-bit sums rounded once per element; it only
   matters for boxes on a clip plane. A parity test against the original
   (`ffx-oracle`) would settle it.
7. **Texture animation** (`m_texAnimID`, the `texswap` textures) and
   other run-time texture changes: the builder binds the cluster's
   texture. The snapshot's live material pointers will tell.
8. **Constant buffer versions and the scene-wide buffer's re-upload**
   (round 2 Q5): the builder makes one constant buffer per draw and
   stage; the game's call stream may update fewer.
9. **Capture formats.** `ffx_scene::capture` reads `capture.ffxgfx` with
   its objects inline; captures whose objects are in a `capture.ffxref`
   (e.g. `orig1/fw-459`) are not read.
10. **Run-time material state beyond the shadow cascades**: bsil00's
    `UVMorphWeight` and animated textures. The snapshot's live material
    buffers give the values; the textures need the live pointers.
