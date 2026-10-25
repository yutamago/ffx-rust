# The scene census on more maps (M4 round 2)

Round 1 (`docs/formats/scene.md`) took the census of one frame of the
salvage ship deck, `cdsp0000`. This page widens it to three more maps and
answers the general questions that page left open: Q1 (texture quality),
Q6 (render orders), Q7 (shadow solutions) and Q8 (unidentified functions).
The answers cite the FFX.exe functions they come from. The names are in
`symbols/ffx.sym`.

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

## Notes for docs/formats/scene.md

The coordinator should carry these over, since this page doesn't edit
that one:

- "Passes": `cdsp0000`'s shadow solution is 2, not 0. Cluster `+0x28c4`
  is in `+0x7c` for 0 and 2, in `+0xbc` for 1 and 3, and in no world for
  4. `PhyreScene_RenderShadow` runs for every solution, but with
  `Shadow=0` it draws nothing.
- "Asset parity", textures: the level rule above (Q1).
- "Sort": the `/1e8` class comes from `m_DObjKind` 1 at map init (Q6).
