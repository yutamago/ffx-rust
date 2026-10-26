# The scene builder: the map world in Rust (M4 round 2)

**Status (2026-10-26).** M4 now ports `PhyreEngine` and the game's glue
function by function instead of rebuilding the scene ("Ported, not
rebuilt", user's decision of 2026-10-25, `docs/rewrite-plan.md`, "M4"). The
builder is not the M4 renderer any more. What still holds:
- It gives the same draws as the original on the captured maps
  (cdsp0000, bsil0000, bsil0300; "Results"), so it stays as a second check
  of the ported render chain and as the seam for M6's backends.
- `FFX_RUST_MAP=build` and its gate tests stay in the tree.
- The facts it found (inputs, culling, constant buffers, the allowed
  sampler difference) describe the original and hold for the ports.

The open items at the end are the builder's own; nothing in M4 waits on
them. Where an item names an original function, its port and oracle test
now exist in the hook (the context variant search, the affine inverse).

`ffx_scene::build::build(clusters, inputs)` makes the draws of the map
world (scene `+0xac`) for one frame: the opaque pass, then the transparent
pass, as `ffx-draw` `DrawState`s and `DrawCall`s, with every object they
use. Its spec is the round 1 census (`docs/formats/scene.md`) and the
round 2 map census (`docs/formats/scene-maps.md`). Crate:
[`crates/ffx-scene`](../crates/ffx-scene).

Its inputs are the live snapshot the hook writes into captures
(`ffx_scene::live`, "Inputs from the snapshot"): nothing is read back
from the captured draws. On cdsp0000 its 111 or 112 draws are the
captured map draws of every capture with scene inputs (`r2a-a`, `r2a-b`:
fw-101 and fw-459, three frames each, hash mode; `wire-cdsp0000-a`, `-b`:
full mode), and on bsil0000 its 92 draws are the map draws of
`wire-bsil0000-a` and `-b`, through `ffx-drawdiff`, apart from one allowed
field (below).

```bash
# the gate from the snapshot (skips without FFX_GAME_DIR and FFX_SCENE_WIRE)
C=~/ffx-work/m4/captures
FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
FFX_SCENE_WIRE=$C/r2a-a/fw-101/capture.ffxgfx:$C/r2a-a/fw-459/capture.ffxgfx:$C/r2a-b/fw-101/capture.ffxgfx:$C/r2a-b/fw-459/capture.ffxgfx:$C/wire-cdsp0000-a/cdsp0000-101/capture.ffxgfx:$C/wire-cdsp0000-b/cdsp0000-101/capture.ffxgfx:$C/wire-bsil0000-a/bsil0000-101/capture.ffxgfx:$C/wire-bsil0000-b/bsil0000-101/capture.ffxgfx \
  cargo test --release -p ffx-scene --test live
# one or more captures: build every frame from its records, diff, report
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-scene --example wirediff -- \
  $C/wire-bsil0000-a/bsil0000-101/capture.ffxgfx --allow crates/ffx-scene/tests/builder-allow.txt \
  --allow crates/ffx-drawdiff/allow/bsil0000.txt [--out rebuilt.ffxgfx]
# captures with the Wire block: tools/replays/draw-<map>.txt (original code, FFX_GFX=1)
```

The round 2 gate from capture-derived inputs (`CaptureInputs`, "Inputs
from a capture") still runs on the round 1 captures, which have no scene
inputs:

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
| `build::Clusters::load(read, model_path)`, `load_models(read, paths)`, `add_textures(read, imports)` | The static data: the world's map models (`map::MapModel`, appended into one with `models` giving each one's instances) and every effect variant (`effect::EffectVariant`) and texture their materials import; textures the game sets at run time (texture animation) are added by import id. `read` maps an archive path to bytes (the archive, or the mod folder first) |
| `frame::SceneInputs` | The game state of the frame: settings, camera, pass names, the bounds' order and visibility jobs, shown flags, worlds and render orders, segment hidden bytes, depth biases, material buffers, the texture in each material slot (`SlotTexture`: an import, a run-time texture, nothing), the scene-wide buffer, viewport and scissor |
| `live::LiveInputs::new(&clusters, &records, fallback_settings, &fallback)` | The inputs of one frame from its `SceneInputs` records and their `Wire` blocks (`inputs::read_wired`) |
| `live::scene_wide`, `live::SCENE_WIDE`, `live::affine_inverse` | The scene-wide buffer of a record, its layout, and `0x004cc890` (the view's inverse) |
| `gate::map_draws`, `gate::run` | A capture's map draws found by their geometry (full and hash captures, morphing meshes), and one gate run (build from the records, emit, drawdiff) |
| `build::build(&clusters, &inputs) -> Result<Built>` | `Built { resources, draws, notes }`: `draws` are `MapDraw { pass, instance, segment, material, state, call }`, opaque first; `notes` name what was not done exactly (per-node semantics left 0, unknown entries) |
| `build::Resources` | The objects, indexed by the handles in the states: shaders, layouts, blend/depth/raster/sampler descriptions, buffers (static vertex and index buffers, one dynamic constant buffer per draw and stage), textures with every subresource, views. A view is a texture's, or `External` (the scene targets, textures the game sets at run time) |
| `upload::Uploaded::new(&mut backend, &resources, host_view)` | Makes the objects on an `ffx-draw` backend; `up.state(&draw.state)` moves a state onto the backend's handles. What the in-game path (R2c) issues |
| `capture_inputs::CaptureInputs::new(&capture, &clusters, settings, msaa)` | Round 2's inputs, read back from a capture (below) |
| `emit::replace_map_draws(cap, built, opaque, transparent)` | A copy of a captured frame with its map draws swapped for the builder's in place (the k-th built draw of a pass for the k-th captured one, so ring updates stay before their draws), host objects taken from the captured draw in that place, in the hash form for hash captures; for `ffx-drawdiff` |

Dev tools (examples): `wirediff` and `builddiff` (above), `scenewide` (the
scene-wide entries of a model's shaders and the `$Globals` member each
fills), `inputs --wire` (a capture's records), `clusters` (a cluster's
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

- The jobs run per instance list of the walk (`SceneInputs::jobs`),
  in runs of 400.
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
| Vertex buffers | Per input, one of the segment's streams of its render type: when the instance's segment context binds the type and semantic index (`PMeshInstanceSegmentStreamBinding`: the water's `TEXCOORD4/6/7`, a second UV set), the one whose `PVertexStream.m_streamSet` (`+9`) is the binding's `m_inputSet`; else the first in block order. The context's bindings are a `PSharray` of pointers, filled through the pointer-array fixups (`Cluster_ApplyPointerArrayFixups` `0x0057acd0`: the array's length; each pointer fixup's sixth word is its element). That block's buffer at offset 0 with its stride. This covers round 1's stand-in: `TEXCOORD1` reads the first `ST` stream. A morphing instance (`m_dynamicMeshInstance` `+0x18` set) reads its `Vertex` stream from the dynamic ring the game writes every frame (`External::DynamicVertices`, the host's) |
| Index buffer | The segment's `u16` indices, `R16_UINT`, offset 0; the call is `DrawIndexed(count, 0, 0)` on a triangle list (`m_primitiveType` 2) |
| Static buffers | One per `PDataBlock` and one per segment's indices: `IMMUTABLE`, bind 9 or 10 (`docs/formats/scene.md`, "Asset parity") |
| Textures | The texture the slot holds this frame (`SceneInputs::material_texture`): the cluster's, or the one the game put there (texture animation, `bsil0000`'s `texswap` frames), by import id; its levels from the one `Phyre_PBinary_loadClusterTextures` (`0x00465ab0`) starts at (`assets::texture_start_level`: `TQ_HIGH` 0, `TQ_MEDIUM` 1, `TQ_LOW` 2 top levels, none from a texture with that few levels past the first, fewer while the new top would be under 4 px), laid out as the upload (`0x0058fc20`) reads the loader's buffer (`upload_subresources`: a side halved to 0 counts as the top level's, so the last levels of a non-square texture are bytes of a dropped level); `BC1/2/3`; cube maps as 6 slices; a `TEXTURE2D` or `TEXTURECUBE` view of every level. A slot with a run-time texture (the shadow maps) binds the host's (`External::MaterialTexture`); an empty slot binds nothing (cdsp0000's `EnvironmentMapSampler`) |
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

A material entry of type `float4x4` (data type and entry kind `0x31`)
goes in transposed: the game keeps a material's matrix as a `PMatrix4`
(column-major), and the cascade matrix `PhyreScene_RenderShadow` writes
into the live buffer (`LightViewProjection`) is its transpose in every
captured map draw. The file's matrices are identities, which hid it. The
node and scene-wide values are built in the shaders' order.

The per-node buffer is the variant's untweakable definitions:

| Semantic | Value |
|---|---|
| 0x82 `World` | the instance's world (cb layout) |
| 0x83 `WorldView` | `View . World`, f32 sums, k = 0 to 3 |
| 0x84 `WorldViewProjection` | `ViewProjection . World`, f64 sums rounded once (the camera's `ViewProjection`, not a product of the builder's) |
| 0x85 `WorldInverse`, 0x86 `WorldViewInverse` | left 0 and noted; no map shader of cdsp0000 reads them |

## Inputs from the snapshot (`live.rs`)

`LiveInputs` takes one frame's map records (the `Opaque*` and
`Transparent*` `renderWorld` calls of world `+0xac`; the `post*` calls
draw nothing of the map) and their `Wire` blocks
(`docs/formats/scene.md`, "Scene inputs"):

| Input | From the record |
|---|---|
| models | each cluster's file (`Wire`), else the model cluster's symbol (`cdsp00`); instances by their slot in the cluster's `PMeshInstance` list |
| bounds order, jobs | the bounds in the visibility check's walk order, grouped by instance list |
| shown | `m_meshInstance` set; a hidden bounds object is tied to its instance through the file's bounds order |
| world, render order | the bounds' `*m_worldMatrix` (the instance's own, Q4) and live `m_render_order` (the `/1e8` class included) |
| hidden segments | each instance's segments' byte `+0x14` |
| depth bias | `g_DepthBias` entries by bounds |
| material buffers | the live `PParameterBuffer` of the material each instance's set holds where the file's set holds the file's material |
| slot textures | the `Wire` block's slots: an asset id is an import, a set slot without one a run-time texture |
| camera | `scene.View` and `ViewProjection` as `ffx_scene::inputs` derives them; the culling matrix `PCamera +0x70` |
| scene-wide buffer | `live::scene_wide`: the layout of `SCENE_WIDE` (the location tables' offsets), `View`, `Projection`, `ViewProjection` from the camera, `ViewInverse` = `PMatrix4_Inverse_004cc0c0` of the view (`live::affine_inverse`, `0x004cc890`'s rounding: cofactors, determinant and the inverse 3x3 stored as floats, the translation as three stored products and two stored sums), `EyePosition` the camera world's translation (`Wire`), `ViewProjection2` (`app +0x90`, transposed), ambient, fog and background colours, fog values and `GlobalTime` from `g_PhyreApplication`, the screen size and its inverse (`fild` then `1 / x`), near, far, `far - near`, `near * far` |
| viewport, scissor | the screen (`0x00c95100`) |
| settings | `Wire`: texture quality, anisotropy |

`PSceneWideParameters_getBuffer` (`0x004fdec0`) computes the same from
its key (`0x004fe940`): the view `FromMatrix3x4`'d, then inverted
(`0x004cc0c0`) for semantic 4; the camera world for semantics 0 and 1;
the renderer's context for semantics 0x10-0x1c.

### What the snapshot doesn't give

| Value | Where the gate takes it | Why |
|---|---|---|
| `EyePosition`, version 1 records only (`r2a-a/b`) | the first map draw's constants | version 1 has no camera world matrix; the view's inverse's translation is 1 ulp off it. Version 1 also has no texture ids: a texture-animated slot binds the cluster's texture (`r2a`'s `besaid-101` differs in that one draw), and no settings (the sandbox's `GameSetting.ini`) |
| the scene targets | the captured draw in the same place | the host's objects (`External::Color`, `Normal`, `Depth`) |
| run-time textures (the shadow maps) | the captured draw's view in the same register | the host's objects; nothing draws into them with `Shadow=0` |
| the morphing meshes' vertices (bsil0000: 13 draws, `FBXASC048..053_0Shape` and others) | the captured draw's ring binding in the same slot | CPU-written each frame by the morph modifiers (`MorphManager`, `PModifierNetworkDynamicMesh`); see "Morphing meshes" |

### Morphing meshes

`bsil0000` and `bsil0300` draw instances whose `Vertex` stream is the
dynamic vertex ring: `PMeshInstance.m_dynamicMeshInstance` (`+0x18`)
points at a `PModifierNetworkDynamicMeshInstance` (22 on `bsil00`), whose
`PModifierNetworkInstance`s run a `PModifierNetwork` (`m_modifiers`,
`m_inputs`, `m_outputs`; the morph targets are the cluster's
`_..._mime-target0Unindexed` objects and the weights
`PMorphModifierWeightsUserDataObject.m_weights`). The game registers
them at map init (`MorphManager_registerClusterMorphs` `0x006aca10`:
`m_mimeCt` 1 with a dynamic mesh `0x006ab8c0`), animates the weights per
frame, and writes the blended positions into the ring before the draw.
The builder makes these draws with every other stream, the index buffer,
states and constants of its own, and binds the host's buffer for the
`Vertex` stream (`External::DynamicVertices`). Building the vertices
would need: the modifier network's code (the morph modifier: base
positions plus weighted target deltas, in the original's float order),
the weights of the frame (`MorphManager`'s animation state, or the
weights array of each `PMorphModifierWeightsUserDataObject` in the
snapshot), and a parity test against the original. `bsil0300` reaches
it the same way: `wire-bsil0300-a` (`tools/replays/draw-bsil0300.txt`)
builds its 80 map draws with the host's vertices, equal to the capture's
but for the water sampler's allow entry.

## In the game (`ffx-hook` `gfx/map_build.rs`)

`FFX_RUST_MAP=build` (with the graphics layer; `tools/run-game.sh
--rust-map=build`, `tools/replays/run-all.sh --rust-map=build`, which passes all 12 traces and the shots like the plain run, 2026-10-25) plugs the
builder into the map path (`docs/draw.md`, "The map path"): for the
supported maps (`SUPPORTED`: `cdsp00`, `bsil00`) the builder draws the
map passes; every other map, and any pass it can't build, goes to the
recorder (`None`).

- **Main thread, update N** (the builder closure, at the map's
  `renderWorld`): other maps leave at once (the model's symbol,
  `PhyreScene +0xd8`). At the update's first map pass the record is read
  as the capture snapshot reads it (`world::scene_inputs_at` on the
  renderer `app +0x124`: the version 1 fields and the `Wire` block) and
  both passes are built; the other pass takes its draws at its own call.
  The reads are made directly (`world::with_direct_reads`: each memory
  region checked once a run with `VirtualQuery`, about 10 a run, instead
  of a `ReadProcessMemory` per read): about 2 ms for the record and 4 ms
  for the update on cdsp0000 and bsil0000 (`ReadProcessMemory` took 10 ms
  a pass, which the realtime replays did not keep up with). The texture
  ids are looked up in the asset tree once per object (per model
  cluster). The map's models are loaded once, by
  their files' ids, through the game's own file functions
  (`Phyre_PSerialization_PStreamFileWin32_openFile` `0x00607f40`, hooked
  for mods, `getFileSize` `0x00607dc0`, `read` `0x00608090`, `closeFile`
  `0x00607d80`; a relative prefix of the root's length, which the archive
  lookup and the mod index skip); textures the snapshot names
  (texture animation) are added the same way. `LiveInputs` takes the one
  record for both passes, and `build_into` builds onto the map's
  `Resources`, so an object keeps its handle from frame to frame.
- **Render thread, frame N** (the job, at the end of the pass):
  `Uploaded::extend` makes the objects new since the last frame (the
  first frame makes them all: 0.3 s for cdsp00's clusters to load, then
  the textures), and each built draw becomes a `PassDraw` (`up.state`,
  its constants as bytes). The host's objects come from the game's own
  draw in the same place: the map path records the draws it drops for a
  built pass (`Job` gets them), and the k-th built draw takes the k-th
  one's scene targets, the shader resources of its external slots (the
  shadow maps) and the vertex buffers of its external streams (the
  morphing meshes' ring). If the counts differ, the game's own draws are
  issued (logged).

Results (2026-10-25, original code otherwise, against `r2a-a`'s and
`r2a-b`'s `draw-field.txt` captures with `~/ffx-work/m4/allow-draw-field.txt`
and `tests/builder-allow.txt`, `--part all`): fw-101, fw-459 (cdsp0000)
and besaid-101 (bsil0000), three frames each, 0 differences in every
builder-mode run (`~/ffx-work/m4/wire/capture-build.sh`,
`diff-build.sh`). The allow list's
`[*]` entry covers the one artifact of building in the game: the device
hands the builder's `CreateSamplerState` back the game's equivalent object,
and the layer records an object's description from the last create, so a
shared sampler shows the builder's `MaxAnisotropy` 0 in every draw that
binds it.

## Inputs from a capture (`capture_inputs.rs`)

For captures without scene inputs (round 1's `fw1..fw3`, round 2's
`cdsp0000-a/b`), the inputs come from the capture through the same
location tables, inverted:

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

From the snapshot (`tests/live.rs`, `wirediff`):

| Capture | Records | Map draws (opaque, transparent) | drawdiff (scene part) |
|---|---|---|---|
| r2a-a, r2a-b fw-101, 3 frames each (hash mode) | version 1 | 100, 11 | 134 items, 0 differences, 7 allowed |
| r2a-a, r2a-b fw-459, 3 frames each (hash mode) | version 1 | 100, 12 | 148 items, 0 differences, 8 allowed |
| wire-cdsp0000-a, -b (`draw-cdsp0000.txt`, full mode; taken again after the character part joined the record) | `Wire` | 100, 11 | 134 items, 0 differences, 7 allowed |
| wire-bsil0000-a, -b (`draw-bsil0000.txt`, full mode) | `Wire` | 71, 21 | 143 items, 0 differences, 1 allowed |
| r2a-a, r2a-b besaid-101, 3 frames each (hash mode) | version 1 | 71, 21 | 143 items, 1 difference: the texture-animated slot (no texture ids in version 1), 1 allowed |
| wire-bsil0300-a (bonus, `draw-bsil0300.txt`) | `Wire` | 76, 4 | 100 items, 0 differences, 1 allowed |

From the capture (`tests/builder.rs`, `builddiff`):

| Capture | Map draws | drawdiff (scene part) |
|---|---|---|
| fw1-a, fw1-b, fw2-a, fw2-b, fw3-a, fw3-b (round 1, user's mods) | 111 | 134 items, 0 differences, 7 allowed |
| cdsp0000-a, cdsp0000-b (round 2 harness, no mods) | 111 | 134 items, 0 differences, 7 allowed |

Building takes about 2 ms; loading the clusters about 80 ms.

### Allowed differences (`crates/ffx-scene/tests/builder-allow.txt`)

| Field | Draws | Cause |
|---|---|---|
| `ps.sampler2.max_anisotropy` (`phyrewatershader`, `CubeMapSamplerState`; bsil0000's water: `ps.sampler1`, ps `91184e0d7f258ef2`, its own `[bsil0000]` entry) | the 7 water draws (bsil0000: 1) | The material's sampler state has `m_maxAnisotropy` 0.0, so the game and the builder pass `MaxAnisotropy` 0. The capture has no sampler object with that description: the device returned the existing object 0x3a311fa0, recorded with `MaxAnisotropy` 1 and otherwise equal (filter 0x15, clamp, `NEVER`). Direct3D 11 hands back an existing state object for an equivalent description, `MaxAnisotropy` does not matter to a non-anisotropic filter, and the graphics layer records an object's description from the last create that returned it (`ffx-hook` `gfx/mod.rs` `create_state`). In-game the builder's call gets the same object (R2c to confirm) |

`SgSpec_CameraVec[0].z`, the junk read named in the round plan, is not in
the map world: its one draw is a character draw (scene draw 112,
`phyrechrlitshader`, the `Ch_DrawMain` / `graphicSgSetSpecular` path), so
the builder's gate has no entry for it.

## Open items

1. **Multi-cluster worlds.** `Clusters::load_models` appends the world's
   models and `LiveInputs` ties every cluster to its model by its file;
   the jobs run per instance list. No captured map has more than one
   cluster in world `+0xac` (cdsp0000, bsil0000 and bsil0300 have one),
   so this path has not met a capture.
2. **Morphing meshes' vertices** come from the host ("Morphing meshes").
3. **The context variant search** (`0x004ef440`) is not in the map
   builder: the first shader of the render pass is taken. (The character
   builder has it, and the hook has its port, `PEffect_findContextVariant`
   in `ported/h2_render_b.rs`.)
4. **Per-node semantics** other than `World`, `WorldView` and
   `WorldViewProjection` are left 0 (noted per build).
5. **The x87 arithmetic of the culling matrix product and the point
   transform** is taken as 53-bit sums rounded once per element; it only
   matters for boxes on a clip plane. A parity test against the original
   (`ffx-oracle`) would settle it, as it would for `live::affine_inverse`
   (exact on every captured frame). The original of the latter,
   `Vectormath_Transform3_inverse` (`0x004cc890`), is now ported
   (`ported/p5_core_c.rs`); the builder still uses its own.
6. **Capture formats.** `ffx_scene::capture` reads `capture.ffxgfx` with
   its objects inline; captures whose objects are in a `capture.ffxref`
   (e.g. `orig1/fw-459`) are not read.
7. **The scene-wide definitions** (`PSceneWideParameters`, renderer
   `+0x5c`) are not in the snapshot: the layout is the location tables'
   (`SCENE_WIDE`), the same in every map shader. Reading `*(renderer
   +0x5c)` at `renderWorldCulledSorted`'s entry did not give the
   definition list.
