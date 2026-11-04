# The scene census on more maps (M4 round 2)

Round 1 (`docs/formats/scene.md`) took the census of one frame of the
salvage ship deck, `cdsp0000`. This page widens it to three more maps and
answers the general questions that page left open: Q1 (texture quality),
Q6 (render orders), Q7 (shadow solutions) and Q8 (unidentified functions).
The answers cite the FFX.exe functions they come from. The names are in
`symbols/ffx.sym`. The last section adds what the game sets up at map
entry and the files leave implicit: how the 2D layers are drawn, where a
map's starting camera is, and the event scripts' placements of map
objects.

## Captures

Each map has a capture session, `tools/replays/draw-<map>.txt`. It runs
one full-mode `gfxcapture` frame on the original code, in fast mode, on the
virtual clock, in lockstep. The `readp` lines log the level-name hash and
the shadow solution. Captures go to `~/ffx-work/m4/captures/<map>-{a,b}`
and never into git.

```bash
FFX_GFX=1 FFX_GAME_DIR=~/steam-games/FFX-X2-HD target/release/examples/replay \
  tools/replays/draw-bsil0000.txt '<windows folder>\draw.trace' original
target/release/drawdiff A/bsil0000-101 B/bsil0000-101 --allow crates/ffx-drawdiff/allow/bsil0000.txt
FFX_GAME_DIR=~/steam-games/FFX-X2-HD target/release/examples/census \
  A/bsil0000-101/capture.ffxgfx /map/bsil/bsil00/ /ps3data/chr/
```

The captures ran with the harness of 2026-10-24 (no user mods, the
sandbox's `GameSetting.ini`: `TQ_LOW`, `Shadow=0`, MSAA 0, HDAO off).

| Map | Reached by | Event count | Level (hash) | Shadow solution | Draws | Scene draws | capture.ffxgfx |
|---|---|---|---|---|---|---|---|
| `bsil0000`, Besaid beach | save 3 | 101 | `/map/bsil/bsil00/` (`0x25e2`) | 4 | 203 | 136 | 126-130 MB |
| `bsil0300`, the river under the cliff (map `0x29`, event-cliff's map) | save 3, event-besaid's walk | 32 | `/map/bsil/bsil03/` (`0x2574`) | 2 | 182-203 | 94 | 140-193 MB |
| `kami0100`, the Thunder Plains travel agency (indoor) | save 5 | 101 | `/map/kami/kami01/` (`0xa5`) | 0 | 187 | 139 | 185 MB |
| `cdsp0000`, the deck (a new pair under today's harness) | save 1 | 101 | `/map/cdsp/cdsp00/` (`0x203b`) | **2** | 197 | 128 | 70 MB |

The hashes and solutions were read in the game, at
`*g_PhyreScene +0xfaf4` and `+0xf9cc`. They match the static table under
Q7. Shadow solutions 0, 2 and 4 are covered. Solutions 1 and 3 belong to
maps that none of the saves reach (Q7).

### The two-run gate

Each map's pair (runs a and b) differs only by its allow list,
`crates/ffx-drawdiff/allow/<map>.txt`. Every entry there gives its cause.

| Map | Items | Differences before / after the allow list | What the list covers |
|---|---|---|---|
| `bsil0000` | 215 / 215 | 1 / 0 | `SgSpec_LightVec[0].z`, the `Ch_DrawMain` junk read. Kept from T2's Besaid list, not seen in this pair: `GlobalTime`, the chocobo icon, the reloaded 16x8 texture, the two-phase ring |
| `bsil0300` | 216 / 195 | 616 / 0 | `GlobalTime` (17 ulp: the walk waits on a real-time voice line), the junk read, the chocobo icon (34 items missing and 13 extra: their number follows the wall clock), and the minimap mesh's `NORMAL0` ring (below) |
| `kami0100` | 196 / 196 | 270 / 0 | `GlobalTime` (3 ulp) |
| `cdsp0000` | 207 / 207 | 252 / 0 | `GlobalTime` (4 ulp) and the junk read (the existing `cdsp0000.txt`) |

The minimap entry (`bsil0300`, vertex shader `af2fbdc4f5970aca`, `vb1`):
the guide map is a CPU-built ring mesh (scissor 97..337 x 56..236, 567
indices). Its `NORMAL0` stream holds colour-like values (k/512: 0.498,
0.249, ...), and one of them differs between the runs in 30 vertices
(0.152 in a, 0.027 in b). It looks like an animated value whose phase is
not tied to ctrl work 0's count. The minimap's pixels are identical in
both runs' screenshots.

## Census per map

The census example's rule checks (`ffx_scene::census`: culling, sort,
matrix arithmetic, asset keys), run on run a. Run b gives the same
results on every map.

| Map | Passes and families (draws) | Bindings | Culling (map cluster) | Sort | Matrices |
|---|---|---|---|---|---|
| `bsil0000` | Opaque: defaultlit 67, chrlit 36, uvanimation 4. Transparent: defaultlit 20, chrlit 8, water 1 | all matched or dynamic or render target; 0 unexplained | 73 instances, 35 drawn, 19 outside the frustum, **19 inside and not drawn** | 37 pairs, 0 out of order | all three rules 136/136 |
| `bsil0300` | Opaque: defaultlit 76, chrlit 8. Transparent: chrlit 5, defaultlit 2, uvanimation 1, water 1, blob shadow 1 | 0 unexplained | 75 instances, 13 drawn, 62 outside, 0 inside | 12 pairs, 0 | 93/93 |
| `kami0100` | Opaque: chrlit 99. Transparent: chrlit 28, prerendered 11, blob shadow 1 | 0 unexplained | the 2D cluster: 22 instances, 11 drawn, **11 inside and not drawn** | 12 pairs, 0 | **`ViewProjection = Projection . View` 11/138** (1 ulp); `WorldView` f32 138/138; `WorldViewProjection` f64 138/138 |
| `cdsp0000` | as round 1 (128 scene draws) | 0 unexplained | 29, 19 drawn, 9 outside, 1 inside (`FBXASC056_0Shape`) | 20 pairs, 0 | 127/127 |

### Census failures the builder must handle

1. **Hidden instances that pass the box test.** On `bsil0000`, 19
   instances pass the box test and do not draw: `FBXASC0498..0512` `_0`
   and `_1`, and `_FBXASC0498Shape`. In the file every one has
   `m_dObjFlag` (`PMeshInstance +0x54`) `0x101` and an `m_objID` of its
   own (`0x12..0x20`). The flag looks like the event-controlled map
   parts' hidden-at-start bit. On `cdsp0000`, `FBXASC056_0Shape` is the
   only instance with `m_groupID` `0xc`, a lead for Q3. On `kami0100`, the
   `scene1layer0..10Shape` instances do not draw: they are the second 2D
   layer group, and `scene0layer*` is shown (`PhyreScene_show2DLayer`
   `0x0066f2f0`, scene `+0x1140`). The builder has to take the hidden
   state from the game (R2a's snapshot: `m_meshInstance` 0, groups,
   layers). The file's flags are not enough.
2. **`kami0100`: `scene.ViewProjection` is not `Projection . View`** on
   127 of 138 draws (1 ulp each). The 11 that match are probably the 11
   prerendered layer draws: same count, unverified. `WorldViewProjection =
   ViewProjection . World` (f64 sums) still holds on all 138. So the
   builder must use the game's `ViewProjection`, or find the input that
   differs. One unverified suspect is the 2D-layer maps' screen shift at
   scene `+0xfa34` (`PhyreScene_getViewProjectionMatrix`) being applied
   to one world's camera. R2a's camera snapshot should settle it.
3. **The texture level rule** (`ffx_scene::assets::texture_start_level`
   = `min(dropped, levels - 1)`) differs from the game's (Q1) in two
   cases. Textures with 2 to `q` levels are not dropped at all. Textures
   whose top level shifted by `q` is under 4 px wide or high drop fewer
   levels. Neither case occurs on these four maps: every bound file
   texture is either 1 level (kept, as both rules say) or drops 2 levels.
4. **Render orders are changed at run time** (Q6): instances with
   `m_DObjKind` 1 get `+100000000`. The census reads `m_render_order` from
   the file, so its sort check cannot see the class. It passes here only
   because those instances already have the smallest orders: `bsil00`'s
   `FBXASC0515_0Shape` (draw 0), and `cdsp00`'s `FBXASC0491/0492/0493`
   (draws 0 and 1).
5. **The shadow targets are never drawn** with `Shadow=0` (Q7). The
   1024 x 1024 targets that `MySMSampler`, `MySMSampler2` and the blob
   shadow's `TextureSampler` sample (`bsil0000`: both shadow samplers 1024
   in 26 draws; `cdsp0000` and `bsil0300`: `MySMSampler` 1024 and
   `MySMSampler2` the 8 x 8 dummy) keep whatever they held since they
   were created. The builder only has to bind them by role.

## Q1: texture quality

`TextureQuality` is settings key 13 (`g_SettingsKeyNames` `0x00c8a248`,
the string at `0x00b7c870`). `Settings_SetValue` (`0x00401930`) stores
the index of the value in that key's table (`g_SettingsValueStrings`
`0x00c89f48` + 13 x 0x20 = `0x00c8a0e8`: `TQ_HIGH`, `TQ_MEDIUM`,
`TQ_LOW`) at `g_Settings +0x34`, so **`TQ_HIGH` = 0, `TQ_MEDIUM` = 1,
`TQ_LOW` = 2**. (`Shadow`, key 12, is "0".."3" at `+0x30`, which
`PhyreScene_ApplySettings` passes to `ShadowManager_SetQuality`.)

`Phyre_PFramework_PApplication_onInit` (`0x0062aa0f`) hands the value to
`Phyre_PRenderInterface_setTextureQuality` (`0x0058e000`, this =
`0x00c94f00`). It is stored at `+0x1340`, and when it changes every
texture file registered at `+0x1344` is reloaded.
`Phyre_PFramework_PApplication_frame` (`0x00627bcc`) calls it again when
the setting changes.

The function that drops the levels is
**`Phyre_PBinary_loadClusterTextures` (`0x00465ab0`)**, called through
the thunk `0x00465aa0` from `Phyre_PSerialization_PBinary_loadCluster`.
`Phyre_PBinary_reloadClusterTextures` (`0x00466da0`) does the same on a
reload. For each `PTexture2D`, with `q` the quality
(`Phyre_PRenderInterface_getTextureQuality` `0x005947d0`) and `m` the
header's `+0xc` (levels - 1):

```text
drop = q
if m >= q:                                  # 0x00466129
    w, h = width >> q, height >> q
    while drop > 0 and (w < 4 or h < 4):    # 0x00466137..0x00466146
        w, h, drop = 2w, 2h, drop - 1
else:
    nothing is dropped                      # m < q
read each level 0..m; levels below `drop` are read into the same place
  (overwritten by the next one), so only levels drop..m are kept
setDimensions(width >> drop, height >> drop, format, m - drop)   # 0x00466218, when drop > 0
```

So `TQ_HIGH` drops nothing, `TQ_MEDIUM` drops 1 top level and `TQ_LOW`
drops 2. Two exceptions: no level is dropped from a texture with `q` or
fewer extra levels, and the drop stops while the new top would be under
4 px wide or high. Round 1's frame agrees: a 2048 file becomes a 512
texture with 10 levels, and 1-level textures (`kami0100` has seven,
1024 to 2048 px) are kept whole. Cube maps were not traced (none of the
four maps samples one).

## Q6: render orders, and lists over 400 bounds

A scan of every map model in the archive (382 `ps3data/map` and 60
`ps3data/btlmap` clusters, `PMeshInstanceBounds.m_render_order`; 33
clusters have no mesh segments):

- **No file value is 100,000,000 or more.** The `/1e8` class is set at
  run time. `MorphManager_registerClusterMorphs` (`0x006aca10`), called by
  `PhyreScene_fieldMapInitialize` on the map and 2D clusters
  (`0x0065bd28`, `0x0065bd3f`) and by `0x0066efd0`, adds 100,000,000 to
  the bounds of every `PMeshInstance` with `m_DObjKind` (`+0x4c`) 1
  (`0x006aca88`), so those sort first in their pass. 134 field map models
  and 29 battle map models have such instances: 1 on `bsil00`
  (`FBXASC0515_0Shape`), 1 on `bsil03`, 5 on `cdsp00` (`FBXASC0491_0`,
  `_1`, `0492_Shape0`, `0493_0`, `_1`), none on `kami01`, 1 on
  `mcfr02`. The census does not see the class (failure 4 above).
- **The thousands field** is used by six field maps. Values: `znkd14`
  2007; `znkd09` 1000, 2000, 2005; `znkd08` 2000, 2032, 2033; `mtgz01`
  2006, 3006, 4006, 5006, 6006, 7006; `mihn07` 1026; `kino04` 2005.
  Everywhere else the orders are 0-117 (battle maps 0-32). Within a job
  the comparator puts `ro % 1000` before the thousands
  (`PCheckVisibleBoundsJob_compareEntries` `0x009f2d70`). So on `mtgz01`
  (orders 0-7 plus 2006 x3, 3006 x3, 4006, 5006, 6006 x4, 7006 x4), the
  16 `x006` instances sort after its two plain order-6 instances and
  before order 7, by thousands.
- **Only `mcfr02` has more than 400 bounds** in one cluster: 602 (one of
  them `m_DObjKind` 1). On that map the visibility check runs more than
  one job (`startJobsForAllBounds` `0x009f2250`), and `mergeResults`
  (`0x009f3660`) merges the job results by the class and the depth key
  only, ignoring both `% 1000` fields. The next largest field clusters
  are `ptkl06` (379) and `znkd08` (364). With the 2D cluster and any
  other cluster in world `+0xac`, their total could pass 400 too, and
  R2a's snapshot gives the true count. Battle maps have 36 at most.
  `mcfr02` is a shadow-solution 1 map that none of our saves reach.
- The captured maps' drawn instances (file orders): `bsil00` 0-15,
  `bsil03` 0-18, `cdsp00` 0-12, `kami01` (the 2D layers) all 0.

## Q7: shadow solutions

`ShadowManager_selectShadowSolution` (`0x006ccd10`, called by
`PhyreScene_fieldMapInitialize` at `0x0065c01c`, result at scene
`+0xf9cc`) is thiscall(sm, scene `+0x93c`, battle flag scene `+0x1196`,
map id `0x01330844`, level hash scene `+0xfaf4`, scenario, scene `+0xd0`).
It returns **2 in every battle**. On the field it switches on the level
hash. The level is `graphicLevelAnalysisName` (`0x00641d00`)'s path piece
after `jppc` and before `bin/`, e.g. `/map/bsil/bsil00/`, hashed by
`hashU` (`0x00716570`) in `graphicLevelSetName` (`0x00641db0`). The
function body, compiled from the dump and run on the archive's 433 field level
paths (`~/ffx-work/m4/r2d/sol.c`), gives:

| Solution | Field maps |
|---|---|
| 0 | 248 maps (`kami01` among them: measured 0) |
| 1 | `genk16`, `lchb18`, `mcfr01`, `mcfr02` |
| 2 | 147 maps: azit03-04, bika00-01, bika04, bjyt00, bjyt02-08, bjyt10, bjyt12, bsil01-07, bsvr01, bsyt01, bsyt05-06, bvyt00, bvyt05, bvyt09-13, cdsp00, cdsp02, djyt00-01, djyt03, djyt06-09, dome00-01, dome03-07, fdmm01, fdmm03, fdmm06-08, genk01, genk15, guad05-07, hiku08, hiku15-16, hiku19, ikai06, ikai08, kami00, kami03, kino00, kino04, kino08-09, klyt01, klyt05, klyt09-12, lchb00-07, lchb15, lmyt01-02, luca01, maca02-05, mcfr00, mcfr03-04, mcfr12-14, mcyt00, mcyt02, mcyt05-07, mihn00, mihn07, msmm00, mtgz06-07, mtgz09-11, nagi04-06, omeg00-01, ptkl01-02, ptkl06, ptkl10, ptkl13, ptkl16, sins02-07, sins09, slik02, slik10-11, ssbt00-03, stbv01, swin00, zkrn00-02, znkd04-07, znkd10 (`cdsp00` and `bsil03` measured 2) |
| 3 | `guad00`, `kino05`, `luca06`, `slik00`, `swin02` |
| 4 | 27 maps: bika02-03, bsil00, bsvr00, bsvr03, genk00, kino01-03, kino07, klyt00, lmyt00, maca00, mihn01-02, mihn05-06, mihn08, mtgz00-02, mtgz08, nagi00-01, znkd08-09, znkd14 (`bsil00` measured 4) |
| 4, then 3 | `mihn04`: 4 before scenario `0xb54`, 3 after |
| 4 or 0 | `bsyt00`: 4 only on map id `0x18a`, with its own cascade constants |

`fdmm0x` and `mcyt0x` share hashes. Solutions 2 and 4 also set the
cascade constants (`0x00c343ec..`, `0x00cdec2c`, `0x00cdec34`, sm
`+0x4020..+0x4040`). Solution 3 sets a per-map callback at sm `+0x4048`.

**Round 1 had cdsp0000 at solution 0. It is 2** (read in the game; the
table agrees).

What each solution does (`PhyreScene_fieldMapInitialize` from
`0x0065c021`, `PhyreScene_RenderShadow` `0x00653600`, `Draw3DWorld`):

- **Ground shadows, cluster `+0x28c4`** (the blob quads,
  `abmapshadercone`/`chry0shadow`; scene `+0xf9d0` set while it is in
  `+0x7c`, `+0xf9d1` while it is in no world). For 0 and 2 it is moved
  into the characters' world `+0x7c` (`Phyre_PWorld_removeCluster`
  `0x00443b70`, `Phyre_PWorld_addCluster` `0x00443250`), and it shows
  up as one transparent draw (`kami0100`, `cdsp0000`, `bsil0300`). For 1
  and 3 it goes to world `+0xbc`, which `Draw3DWorld` renders after the
  map's transparent pass. For 4 it is removed from whichever world holds
  it (`0x0065c02c..0x0065c04a`), so `bsil0000` has no blob shadow
  draw.
- **Shadow maps**, from `PhyreScene_RenderShadow`. For 2, 3 and 4 it
  runs the cascade pass `0x006c9e50`, then puts the cascade matrices and
  samplers into material buffers (`LightViewProjection`, `LVP2`,
  `MySMScaleBias*`). For 4, cascade 1 becomes `MySMSampler2` (the dummy
  map in battle). For 0 and 1 it runs only `0x006ce6f0`. Then, for every
  solution but 4, `0x006caee0`. **Both `0x006c9e50` and `0x006caee0`
  return at once while ShadowManager `+0x4060` (the `Shadow` setting) is
  0** (`0x006c9e80`, `0x006caf13`). With the sandbox's `Shadow=0`
  nothing is ever drawn into the shadow targets: every reference capture
  has no shadow-pass draws.
- What they draw with shadows on. A probe set `g_ShadowManager +0x4060` to
  3 at run time (`~/ffx-work/m4/r2d/probe-shadow*.txt`; not references):
  - solution 2 (`cdsp0000`): 17 draws before the scene. A full-target
    clear quad, then the casters (skinned character segments, 13 draws)
    into a 1024 x 1024 target at a 960 x 960 viewport, then three
    fullscreen blur passes between it and a second 1024 target;
  - solution 4 (`bsil0000`): 47 draws before the scene, two cascades.
    The characters go into one 1024 target and the beach's skinned map
    objects and NPCs into another, each blurred the same way;
  - solution 0 (`kami0100`): 145 draws. Eight clear quads, then every
    character in its own tile (viewports of about 70-128 px) of a
    1024 x 1024 atlas, then a blur. This is the texture the blob shadows
    sample (`TextureSampler`, 1024 x 1024 render target).

## Q8: unidentified functions

- **`0x006ada20` = `RenderDelegationHelper_beforeScene`** (tentative). It
  runs the render delegations whose flags have `0x8`, under the mutex
  `0x00cccaa0`. One-shot entries return to the free list
  (`RenderDelegationHelper_setDataToFreeList`). `PhyreScene_render` calls
  it after `PhyreScene_RenderShadow` and `0x0066b560`, before the scene
  targets are cleared (`0x00668ef0`). The name follows the Switch build's
  declaration order (beforeUpdate, afterUpdate, beforeScene,
  beforePreRender, beforeRenderWorld, afterRenderWorld), which matches the
  flags and call sites: `0x2`/`0x4` run in the update (`0x00657140`), and
  `0x10`, `0x20` (just before `Draw3DWorld`) and `0x40` (after the
  worlds) follow in `PhyreScene_render`.
- **`0x00695ba0` = `PostProcessManager_renderHDAOPass`**, the HDAO pass.
  `PhyreScene_render` calls it on the post-process manager after the map
  and character worlds (`0x0066954c`). It does nothing unless `+0x123d0`
  is set and the HDAO setting (`+0x1264`, `g_Settings +0x48`) is on; the
  sandbox has HDAO off. It resolves depth and sets the HDAO shader's
  `g_numRing`, `g_kernelScaleChr`, `g_rejectRadiusChr`,
  `g_acceptRadiusChr`, `g_intensityChr`, `g_blurDistanceChr` and
  `g_fadeoutScale`. This is likely the Switch build's
  `PostProcessManager::renderHDAO`. Fahrenheit's call-graph match put
  that name on `0x0066b560`, which is a PhyreScene fastcall rendering with
  an orthographic camera into scene `+0xfc70`/`+0xfc74`; the `fh` line is
  unchanged.
- **`0x005bafc0` = `Phyre_PRendering_PRendererBase_setClearStencil`**. It
  queues render command 10. The render thread
  (`PRenderThread_handleNonRenderInstanceCommand`, case 10, `0x00599ec0`)
  stores the value at `+0x390`, the stencil byte that
  `ClearDepthStencilView` (device context vtable `+0xd4`, `0x0058e4a0`)
  clears to. `PhyreScene_render` sets 0 before the scene clears
  (`0x00668fc2`) and before the 2D pass (`0x0066aa6a`). Its neighbour
  `0x005baf10` is `setClearDepth` (command 9, `+0x38c`).
- **`renderMeshInstanceList`'s multithreaded branch never runs** in the
  field. `Phyre_PRendering_PRendererBase_renderMeshInstanceList`
  (`0x005b3df0`) takes it only when `Phyre_PThreadPool_GetThreadCount`
  (`g_PThreadPoolThreadCount` `0x00c0b950`) is not 0 *and* equals the
  count of the renderer list it is passed. That list is
  `*g_PhyreApplication +0x48` (`PApplication_renderWorld` `0x0062be70`
  passes `app +0x40`, and the function reads `+8`). Read in the game on
  `cdsp0000`: thread count 1, list count 0, so the single-threaded loop
  runs.

## Map entry state: 2D layers, script placements, starting cameras

Found on 2026-11-03/04 by ffx-remix, a separate modding project by the
same author, while its offline map viewer was made to show maps as the
game does (its M3 review; `native/crates/ffx-remix-native/src/scene.rs`
and `event.rs`, the notes in `~/ffx-work/ffx-remix/m3/review1/`). The
facts below come from FFX.exe functions already named in `ffx.sym`, the
map files, and three captures of the original (`kami0100-a`,
`cdsp0000-a`, `bsil0000-a`). Counts are over the archive's 329 field
maps. What was not checked is marked so.

### The `YNSC` layer record

`rcbgSceneLoad` (`0x0092b5e0`) reads a `YNSC` chunk of `MAP1` section 0
(`docs/formats/field.md`, "Background files"): the layer count, 12
bytes, the layers (`rcbgLayerLoad` `0x0092b1a0`), then the scene's
camera block (below). A layer starts with a 0x90-byte record:

| Offset | Type | Field |
|---|---|---|
| `+0` | `u32` | flags (`0x8000` on the layers read) |
| `+4` | `i32` | width, PS2 pixels |
| `+8` | `i32` | height |
| `+0xc` | `i32` | x on the 512 x 448 PS2 screen |
| `+0x10` | `i32` | y |
| `+0x14` | `f32` | depth |

The record is followed by the frame count (16 bytes), the frames (4
bytes each, padded to 16) and their transfer, if any, then 16 bytes and
the image transfer (a transfer is 32 bytes plus count x size, the words
at `+0x18` and `+0x1c`).

- **Depth.** For 1,035 of the 1,075 layer instances `+0x14` is the z of
  the layer mesh's vertices in the PC file (`scene<s>layer<l>` in
  `2d/mdl/d3d11/`). The other 40 differ, probably layers the remaster
  retouched. Smaller is nearer.
- **x, y.** On `kami0100` layer 2 they are 328 and 42 for a layer that
  does not fill the screen, so they look like the top-left corner in PS2
  pixels. That reading is **not checked** against the layer meshes' x
  and y.

### How the game draws the 2D layers

- **The pass.** `PhyreScene_DrawNonUIPrerenderScene` (`0x006507d0`;
  port: `draw_non_ui_prerender_scene` in
  `crates/ffx-prender/src/h3_glue43.rs`) renders the prerender world
  (scene `+0x8c`, the 2D model's instances) with camera `+0x7e0`, before
  the 3D world: `preOpaque`, then `preTransparentATC`, or
  `preTransparent` on four maps (`PLAIN_TRANSPARENT_MAPS`: `0xa0`,
  `0xa1`, `0x101`, `0x16c`). Then it puts camera `+0xdc` back on the
  renderer.
- **The layer camera.** `PhyreScene_initScene` (`0x0065f450`; port:
  `init_scene` in `crates/ffx-prender/src/h3_glue59.rs`) sets up camera
  `+0x7e0`: aspect `0x3f924925` (8/7), near 0.001 (`0x3a83126f`), the
  main camera's far (65536 on the field), `+0x154` = 512 / aspect x 2.5
  x 0.5 = **560**, and as its world the start matrix turned by pi about
  y. The port's comment calls it "the light camera". It is the
  **orthographic** camera of the 2D layers: in the `kami0100-a` capture
  the layer draws' `View` is `x' = -x`, `z' = -z - 1` (turned by pi
  about y, at z -1) and their projection scales x by 1/640 and y by
  1/420 (560 x 3/4, the squeeze below). With the 2D placement (`(-256,
  -224)`, then `diag(-2.5, -2.5, 1)`;
  `PhyreScene_doOnFieldMapAssetLoaded`, `docs/formats/scene.md` Q4) the
  512 x 448 screen fills ±640 x ±560, so a vertex's x and y are its PS2
  pixel whatever its z. The camera's turn undoes the placement's x
  mirror.
- **The 16:9 window.** Unless `+0xf9c7` or `g_PhyreScene3D` is set, the
  pass translates the projection by (`+0xfa34`, `+0xfa38`) and squeezes
  it as `PhyreScene_GetScreenAdjustedView_00651650` does
  (`docs/formats/render.md`, round 20): y x 4/3, then moved by
  `d = 1/3 - 2 (+0xf9bc + +0xf9c0)`. So 336 of the 448 rows show, 16:9
  once the PS2's 7/6-wide pixels are taken, and the top row shown is
  `112 - 336 (+0xf9bc + +0xf9c0)`: the game pans a 16:9 window over the
  picture.
  `+0xf9bc` follows the player's height on the screen
  (`PhyreScene_getChaPos` `0x0065d860`), clamped to 0.035..0.295 (the
  `.rdata` constants `0x00b3eb40`, `0x00b43d00`), and is 0.035 while
  `+0xf9c4` is clear (`PhyreScene_change2DSceneNum(1)` sets it). In the
  `kami0100-a` capture it is 0.275, which shows PS2 rows 19.6..355.6.
  The ffx-remix notes also read 0.285 and 0.2 on a few events and
  `+0xf9c0` = 0.23 on level `0xec1`, -0.01 on `0xc58`; those are not
  re-traced here.
- **Depth and order.** The layers' vertex shader
  (`phyreprerenderedshader`, all five variants the maps use) takes a
  position as `(x, y, -10 z, 1)` (`mul r0.xyz, v0.xyz, (1, 1, -10)`). x
  and y go through `WorldViewProjection` (the layer camera), but the
  output depth comes from rows 2 and 3 of `scene.Projection`, the
  **main** camera's projection (near 1, far 65536, the field camera's
  field of view). So a layer gets the field camera's depth at 10 times
  its file depth, on the same scale as the characters drawn after it,
  and the depth test (`LESS_EQUAL`; cut-outs write depth, blended layers
  do not) draws the layers in file-depth order, nearer over farther,
  whatever the view. The layers' world bounds sit at z -10 x depth, not
  at the mesh's z, which matters for anything that culls or picks them.
- **Scenes.** A map's `YNSC` scenes are alternative pictures of the same
  room (`kami0100`: two scenes of 11 layers each). After `rcbgLoadBin`,
  `Yn_PreBgSetData` (`0x0091aa60`) calls `rcbgPreSetSceneNo(0)`
  (`0x009217e0`): it sets scene 0's camera (`rcbgSceneSetMatrix`) and,
  through `graphicChangeSceneNum` and `PhyreScene_change2DSceneNum`
  (`0x006587d0`), shows only the layers whose name holds `scene0`. So a
  map **starts on scene 0** and shows one scene at a time. Scripts
  change it later (`yiSetPreBgPage` `0x0090a6f0` → `Yn_PreBgChangeStage`
  `0x0091a0d0`). This is census failure 1's `kami0100` case above.

With these rules ffx-remix's offline frame of `kami0100` (the start
scene, the layer camera, the capture's bias) matches the game's
screenshot of `kami0100-a` to the pixel: mean difference 5.05/255,
median 0, 7.8% of pixels off by more than 8 (characters, the minimap,
objects on the counter, post-processing).

### Map cameras in the files

A map's starting camera is in one of two places.

- **The `YNSC` camera block** (2D maps). After a scene's layers: eye,
  look-at and up (16 bytes each), the field of view in degrees (`f32`)
  and a y offset (`i32`). The roll (`+0xac` of the scene in memory) is
  not in the file. The values are field units before the scene's `+0xf4`
  scale (10, `rcbgSceneInit`). **79 maps** have one or more blocks with
  a field of view (54 one, 11 two, 9 three, 3 four, 2 five; `kami0100`
  two, 43.52 degrees): 51 maps of 2D layers only and 28 with a 3D model
  too. Other 3D maps have none or a zeroed one. `rcbgPreSetSceneNo(0)`
  starts on scene 0's block. The block is the main (field) camera, the
  one the characters are drawn with; the layers themselves are drawn
  with the layer camera above. In 3D, ffx-remix's reading is: eye and
  target x 10 (`rcbgSetViewMatrix`, `rcbgScaleVector`) with x and y
  negated (as `graphicCameraSetWorldMatrix` turns the PS2 view), up with
  x and y negated, vertical field of view `2 atan(0.75 tan(fov / 2))`
  (the `kami0100-a` main camera's projection y, 3.34032, agrees), and
  the y offset as a shift of the screen centre by that many of 448 rows
  (`rcbgSetProjectionMatrixSub`). That reading is **not checked against
  a capture** of any of the 28 3D maps, and how the PC renderer uses the
  y offset is open.
- **The event script's camera worker** (3D maps). `AtelRequestMapCamera`
  (`0x0086ed80`) signals the camera worker (script block `+0x18`) with
  the camera entry of the current entrance's record (`+6`; the record
  also holds the yaw at `+8` and the position at `+0xc..+0x14`,
  `docs/formats/script.md`). When that entry's straight-line start puts
  the reference on the player (`Native_CamRefSetTarget`, `0x6027`:
  target, bone, y offset) or on a point (`Native_CamRefSetPos`,
  `0x6020`), the eye around it (`Native_CamSetPolar`, `0x6004`: theta,
  phi in degrees, length) and the zoom (`Native_CamSetScreenDepth`,
  `0x603b`), the follow camera can be read without running the game. On
  `cdsp0000` (camera worker 27, entry 6) the `cdsp0000-a` capture's
  camera has exactly the script's angles (-116.33, -10.33) and length
  (88.72), and a field of view of 46.10 degrees for depth 564, which is
  `2 atan(240 / depth)`; the 240 is fitted to this capture, not traced.
  The eye is `target + length (cos phi cos theta, sin phi, cos phi sin
  theta)` in field units, then `(-x, -y, z)` into the world. **29 field
  maps** have such an entry for entrance 0 (12 of the 177 maps with only
  a 3D model); 10 of them also have a `YNSC` block, so **19** get their
  starting camera only from the script. The other camera entries use
  fixed eyes (`Native_CamSetPos`, `0x6002`, about 30 maps; their axes
  are not checked, and views built from them looked wrong, e.g.
  `mihn0600` upside down), eye rails (`0x6006`-`0x6008`, 64 maps,
  meaning unknown) or focus lists over script data (about 100 maps).

### Script placements at map entry

Map objects that a script binds and moves are placed by the event script
at map entry, not by the model file: in the file they sit at the origin.

- A worker binds itself to a map group (`Native_0004_WorkerSetType4`:
  `+0xa8` the group, `+0xaa` 4) or a display object
  (`Native_0197_WorkerAttachDobj`: `+0xa8` the object, `+0xaa` 5) in its
  init entry (entry 0), then sets a position (`Native_SetPos_0126`) and
  angles (`Native_SetPitch` `0x0096` into position block `+0x30`,
  `Native_SetYaw` `0x0095` `+0x34`, `Native_0183_StopTurnSetAngle38`
  `+0x38`).
- `AtelSetBindObject` (`0x0086f4c0`) hands them to `yiSetRotTransGroup`
  (`0x0090a830`, kind 4) or `yiSetRotTransDObj` (`0x0090a750`, kind 5),
  which reach `PhyreScene_setObjectTranslation` and
  `PhyreScene_setObjectRotation` (`0x0066e3e0`, `0x0066db00`;
  `docs/formats/render.md`, "Object translation/rotation"). Every mesh
  of the object gets the translation `(-x, -y, z)` and 10 x the turns;
  the file's node world is dropped.
- The PC model's `PMeshInstance` has the class fields `m_objID` (the
  object a script names) and `m_RotType` (a turn order). ffx-remix uses
  `m_RotType` as the `order` argument of `setObjectRotation` (0 z y x
  ... 5 x y z). Turns about one axis are checked against the capture;
  the order of a product of several turns, and that `m_RotType` equals
  the DObj's order word (`+0x76`, which `yiSetRotTransDObj` passes), are
  **not checked**.

Read from straight-line init entries with constant arguments (through
`ffx-script`'s disassembler), **105 field maps** have such placements:
554 workers, 427 binding a group and 127 a display object. Positions the
script computes, entries that branch first, and the motion after entry
(the workers' loops, the `MAP1` keyframe animations of
`yiAnimAttachDefault`) are not read that way.

Example, `cdsp0000`: the winch wheel `FBXASC051_0Shape` (`m_groupID` 13,
`m_objID` 3) is at the origin in the file, and `MAP1` section 1's object
3 has a zero transform and no default animation. Worker 25's init entry
binds group 13 (`WorkerSetType4(13)`), then `SetPos(0, -12.8, 196.7)`
and `SetYaw(0)`; its entry 1 loops `TurnSetPitch`. In the `cdsp0000-a`
capture the wheel's world translation is `(0, 12.8, 196.7)`, turning
about x. The hatch (`FBXASC053_0Shape`, worker 22) and the crane arm
(`FBXASC057_0Shape`, worker 23) get the capture's translations the same
way, and `FBXASC056_0Shape` (worker 24) the x = +123.7 of R2a's answer
to Q3 in `docs/formats/scene.md`. `cdsp0200` has the same wheel
(`FBXASC0490_0Shape`).

## Notes for docs/formats/scene.md

The coordinator should carry these over, since this page doesn't edit
that one:

- "Passes": `cdsp0000`'s shadow solution is 2, not 0. Cluster `+0x28c4`
  is in `+0x7c` for 0 and 2, in `+0xbc` for 1 and 3, and in no world for
  4. `PhyreScene_RenderShadow` runs for every solution, but with
  `Shadow=0` it draws nothing.
- "Asset parity", textures: the level rule above (Q1).
- "Sort": the `/1e8` class comes from `m_DObjKind` 1 at map init (Q6).
