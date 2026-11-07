# Effect draws: what PhyreEngine draws for the effects, and from what

The M4 census of the effects (`docs/rewrite-plan.md`, M4; the scene's was
`docs/formats/scene.md`): every effect draw of captured battle frames,
traced to the `VFXDynamicGeometry` it draws, the effect object that made
it (ppp object or op record), the `PhyreScene` list and pass that draw it,
its shader, states and constants. It is the spec for the effects part of
the Rust scene builder. Crate: [`crates/ffx-fxdraw`](../../crates/ffx-fxdraw).

```bash
# one frame: every draw with its producer, the effect state table, the gate
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-fxdraw --example census -- \
  ~/ffx-work/m4/captures/fx-magic/battle-magic-2 ~/ffx-work/m4/captures/fx-magic/battle-magic-2.snap
# the gate over every <name>/ + <name>.snap pair of the folders (skips when unset)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
  FFX_FXDRAW_CAPTURES=~/ffx-work/m4/captures/fx-magic:~/ffx-work/m4/captures/fx-overdrive \
  cargo test --release -p ffx-fxdraw --test captures
```

Other examples: `draws` (a capture's draws with targets, shader families
and ring slots), `probe` (where a draw's ring bytes sit in a snapshot),
`peek` (words of a snapshot; `find` for a word).

## The captures

`tools/replays/draw-fx-magic.txt` and `draw-fx-overdrive.txt` are
`battle-magic.txt` and `battle-overdrive.txt` up to their last effect shot,
with a hash capture (`gfxcapture … hash frames 1 now async`) and a memory
snapshot (`snapshot`) at each effect shot: battle-magic 1-3 (magic 481,
the `op_shape_draw` burst) and battle-overdrive 2 (Spiral Cut), 3 and 4
(Valefor's entry, magic 0x253) and 6 and 7 (Valefor's overdrive). Original
code (`original`), graphics layer on (`FFX_GFX=1`), sound off.

The snapshot is taken one frame after the capture's held point (`wait 1`,
then `snapshot`): the effect vertices a captured frame draws are those of
the update after the held point. A 3-frame capture against snapshots taken
0, 3 and 4 frames after the held point showed it: frame k's effect draws
match the snapshot taken k + 1 frames on (frame 2 against the 3-frame
snapshot: 205 of 212 traced at once, against 22-43 for every other pair).
`PhyreScene_render` merges the frame's VFX arrays itself
(`PhyreScene_MergeVFXRenderArrays` `0x00654650`, render thread), while the
main thread runs the next update.

**On PC2** (2026-10-25) the replays' saves are missing (the main
machine's `ffx_001` is `cdsp0000`, its `ffx_005` `kami0100`; here slot 1 is
story 3210 at Macalania). The captures above were taken with variants of
the two scripts (`~/ffx-work/m4/pcfx-scripts/`, not in git): save 1, the
party written (Rikku, Tidus, Wakka for the magic; Yuna and Tidus for the
overdrive), battle `genk16_03` on land for both, Tidus using the first
item of his list (a Hi-Potion: item 1's effect IDs, table `+0x84`, made 481
as item 0's), Valefor's overdrive matched by the ID's high byte, and the
load waits raised to 300000 frames (a battle took up to 17000 lockstep
frames to load here). Same effects, other party, map and timing: the
numbers below are these captures'. The scripts in git are for the main
machine's saves.

## Where the effects are drawn

`PhyreScene_render` (`0x00668930`), for these battle frames, in this order
(draw indices of battle-magic-2; the census's "runs"):

| Draws | Producer | Pass |
|---|---|---|
| 0-65 | the 3D scene (map, characters, monsters, blob shadows) | Draw3DWorld into the scene targets (`docs/formats/scene.md`) |
| 66-78 | post | the glow chain (`phyreglow.fx`, 13 draws) |
| (64-70 in overdrive 6/7) | late world | the world at scene `+0x9c` and the postOpaque/postTransparent passes, into the colour target alone, after the glow: Valefor in its overdrive (7 `phyrechrlitshader` draws) |
| 79-284 | **effects** | `PhyreScene_DrawMostVFX` (`0x00650670`) |
| 285-286 | post | `copybuffer.fx`, `brightnessshader.fx` (`phyreblur.fx` and `phyredistortionshader.fx` in some frames) |
| 287-353 | UI | `menudefaultshader*.fx`, the fonts |
| 354 | post | the final copy to the back buffer |

Every effect draw of the eight frames is in one block, the
`DrawMostVFX` pass: Transparent pass name (`0x00c169d4`), into the scene
colour target (`1280x720` R8G8B8A8, view format 28) against the scene
depth (R24G8, depth test on, write off), one render target only (the
normal target of the scene pass is not bound).

### The VFX lists of `PhyreScene`

A `VFXDynamicGeometry` is put on a list every frame by its update
(`VfxGeometry_Register_006a52f0`, vtable `+8`, called by
`graphicVFXUpdateWithType` `0x00645ba0` with the type in its flags):

| Kind (flags) | List | Drawn by |
|---|---|---|
| 0, flag bit 31 set (`graphicVFXPath3Update`) | `+0x38cc` (fixed, 0xc00 entries, count `+0xf8cc`): the mesh instance (`geometry +0xc`) | `PhyreScene_DrawMostVFX` |
| 0, bit 31 clear | `+0x68cc` (count `+0xf8d0`): the geometry itself, to be depth-sorted | merged into `+0x38cc` by `PhyreScene_MergeVFXRenderArrays` |
| 0, geometry byte `+0x50` set | `+0x3880` (count `+0x3878`) (`PhyreScene_AddVFXToList3880` `0x006565e0`) | no pass found in `PhyreScene_render` (open question 4) |
| 1 | `+0x3868` (count `+0x3860`) (`0x006a0d40`) | `PhyreScene_DrawUI2DVFX` (`0x00650ea0`), only while fading |
| 2 | `+0x385c` (count `+0x3854`) (`0x00656990`) | `PhyreScene_RenderDTTVFX` (`0x006531b0`) |
| 4 (`0x00ccca58 == 1` and kind != 2, or geometry `+0x3c` bit 8) | `+0x3888` (count `+0x388c`) (`0x006567e0`) | `PhyreScene_RenderVFXAfterPostProcess` (`0x006538d0`) |

Bit 30 of the flags skips the geometry's own update (vtable `+0x28`,
`VFXDynamicGeometry_updateRenderDataWithoutAddingToArray`).

`PhyreScene_MergeVFXRenderArrays` (`0x00654650`) closes the current sync
path (`PhyreScene_VFXSyncPath` `0x00654cc0` opens them; counts at `+0xf8d4`
and `+0xf914`, `+0xf954` paths) and rebuilds `+0x38cc` path by path: that
path's unsorted mesh instances first, then its sorted geometries' mesh
instances. The sort (`PhyreScene_GetVFXSortMode` `0x006721b0`): mode 1 (the
default, and 24 magic IDs) puts the geometries into 4096 buckets by `ftol`
of their depth (geometry `+0xdc`, f32, -1 from the constructor), each
bucket in insertion order; mode 2 (magic 0x203, 0x29b) is the CRT `qsort`
with `VFXGeometry_CompareDepth` (`0x00659140`); mode 0 (five level hashes)
leaves them. `PhyreScene_AddDrawEntry` (`0x00656c10`) sets `+0x10d2c` when
the mode is not 0.

`PhyreScene_DrawMostVFX` then renders the list in order: each non-zero
entry through `renderMeshInstance(mi, mi +0x10, 0, scene +0xd0 +0x13c)`,
with the draw events (`PhyreScene_addVFXDrawEvent` `0x00656940`, 16-byte
records at `+0xfb5c`, count `+0xfb54`) run when their position (`+4`, the
list count at the time of the event) is reached
(`PhyreScene_processVFXDrawEvent` `0x00666e40`: 0 characters with the
event camera, 1 copy the colour or depth buffer, 2 screen fade, 3
PS2-lighting set-up `0x006e5a40`, 4 specular, 5 draw a character, 6 a
z pre-pass). For magic 0x296 the geometry of instance (0x78, 0x5bc) is
held back and drawn last. No frame here had an event.

A mesh instance draws one indexed draw per segment of its render data
with indices (segment `+8` > 0), in segment order.

### What the census checked

The census builds that sequence from the snapshot (every list entry, the
geometry whose `+0xc` is the entry, its segments with indices) and aligns
it with the captured effect draws (draws with the VFX vertex layout or a
`vfx*.fx` shader), a pair counting only when the draw's ring bytes are the
segment's: the four vertex arrays from the draw's first record and the
indices relative to their smallest. On all eight frames:

- every expected draw is drawn, in list order, and every effect draw is an
  expected one, but for 24 draws (battle-magic-1: 7, battle-magic-3: 17)
  whose list entry the snapshot holds as 0: their geometry sits in the
  table's delete pool (destroyed by `graphicVFXDestroy`; after the frame's
  render `PhyreScene_doDeleteMeshInstances` zeroed the entry,
  `PhyreScene_removeVFXFromRenderArray` `0x006687a0`, before the
  snapshot), with the
  bytes the frame drew, at a zeroed entry between its neighbours' places;
- only `DrawMostVFX` was in use (the other four lists were empty).

## The geometry

`VFXDynamicGeometry` (constructor `0x0069e3e0`, vtable `0x00b47e40`), what
the census reads:

| Offset | Meaning |
|---|---|
| `+0x0c` | the mesh instance the lists hold |
| `+0x20` | the shader (`ClassDynamicGeometry_SetShader_006a4800`) |
| `+0x24` | the render data (`docs/formats/render.md`): `+0x90` segment count, `+0x94` segments of `0x6c` bytes |
| `+0x2c` | the cluster the shader came from (default `g_PhyreScene +0x2874`) |
| `+0x3c` | the VSP flags (below) |
| `+0x50` | byte: list `+0x3880` instead |
| `+0xdc` | f32 depth key of the sort (-1 at construction) |

A segment's vertex arrays (`graphicDynGeoMemManagerGetBufferForSegment`
`0x0063fdb0`): `+4` vertices, `+8` indices, `+0xc` positions (12 bytes a
vertex), `+0x10` normals (12), `+0x14` colours (16), `+0x18` uvs (8),
`+0x1c` indices (u16); the same at `+0x2c + 0x1c n` for buffer slot `n`.
The draw binds them as rings: positions slot 0, normals slot 1, colours
slot 2, uvs slot 3 (strides 12, 12, 16, 8; `POSITION0` f6, `NORMAL0` f6,
`COLOR0` f2, `TEXCOORD0` f16), the indices as a ring `R16_UINT` buffer, the
bytes as the arrays hold them (the census compares them byte for byte).
In these frames every drawn segment used its own arrays (slot `+4`).

The table (the one `g_VFXRenderDataTable` `0x00ccca6c` points to,
`docs/formats/render.md`)
also keeps a delete pool: count `+8` (bit 31 a flag), keys `+0xc` (8 bytes
each), counters `+0x14`, geometries `+0x1c`
(`ClassVFXRenderDataTable_UpdateDeletePool` `0x0069fe50` counts them down
and clears their segments).

## Shaders

The shader is picked by the geometry's VSP flags (`+0x3c`): 
`VfxShaderManager_FindShader` (`0x006c06e0`) looks the texture name up in
the level's or spell's special-shader table first, then the flags in the
table that `VfxShaderManager_loadVfxShader` (`0x006c0fc0`) builds from
`ffx_data/gamedata/ps3data/shaders/vfxShader_fx.txt` (`Name.fx.phyre:VSP_A|VSP_B`
lines, the bit values in the file's own comment block: `VSP_ZMAT` 1,
`VSP_ALPHA_ADDITIVE` 2, `VSP_ENABLE_TEXTURE` 8, `VSP_ALPHA_SUBSTRACT` 0x10,
`VSP_ALPHA_SUBSTRACT2` 0x20, `VSP_ZWRITE_ENABLE` 0x40, …;
`VfxShaderManager_loadShader` `0x006c0a70`). With no match,
`ClassDynamicGeometry_SetShader_006a4800` retries with only bit 8 kept. The
census reads the same file from the archive: every effect draw's pixel
shader is the file's shader for its flags (0 mismatches, 0 unlisted flags).

| Flags | Shader (pixel; vertex) | Draws |
|---|---|---|
| `0xa` additive, texture | `vfxadditiveshader.fx`; vertex shader shared with `vfxadditivenclipshader.fx` | 836 |
| `0x8` modulate, texture | `vfxdefaultshader.fx` | 149 |
| `0x18` subtract, texture | `vfxsubstractshader.fx`; vertex `vfxdefaultshader.fx`'s | 46 |
| `0x9` zmat, modulate, texture | `vfxshapeshader.fx` | 10 |
| `0x2` additive | `vfxadditiveshadernt.fx` | 1 |

(The family of a shader is the first archive cluster holding its bytecode;
several clusters share vertex shaders.) The field capture `fw1-a` adds
`vfxadditiveshadernt`, `vfxzmatadditiventshader`/`vfxshapeshadernt` and
`vfxnzadditiveshadernt` (one draw after the UI: another list, open
question 4).

## States and constants

From `ffx-drawdiff`'s decoding (Direct3D 11 enum values), per flags and
pixel shader, over the eight frames:

| Flags, shader | Blend (src/dest/op, alpha) | Depth | Raster |
|---|---|---|---|
| `0xa` `vfxadditiveshader` | 2/2/1 (one, one, add) or 5/2/1 (src alpha, one, add); alpha 2/1/1 | test on, `LESS_EQUAL` (4), write off | cull none (1) |
| `0x8` `vfxdefaultshader` | 5/6/1 (src alpha, inv src alpha) or 2/6/1; alpha 2/1/1 | the same | the same |
| `0x18` `vfxsubstractshader` | 2/2/3 (one, one, reverse subtract) | the same | the same |
| `0x9` `vfxshapeshader` | 5/6/1 | the same | the same |
| `0x2` `vfxadditiveshadernt` | 2/2/1 | the same | the same |

Write mask 15. One flags value can come with two source factors (open
question 2).

What `vfxadditiveshader` computes (ffx-demaster `docs/effects.md`, from
its DXBC through `ffx-dxbc`): the vertex shader doubles the vertex colour
(PS2 `0x80` = 1.0), the pixel shader writes `tex.rgb × colour.rgb` and
`tex.a × colour.a × fades`, each clamped to 1 by the 8-bit target. The GS
clamps `Ct × Cv` to 1.0 as well, but `At × Av` only at `0xff`, which is
2.0, so on the PS2 a vertex alpha above `0x80` can still double a
texture's colour in the `Cs × As` blend and burn it out to white; on the
PC the alpha side stops at 1.0. The HD effect textures store most glows,
trails and rings (greyscale on the PS2, alpha `0x80`) as white with the
brightness in alpha, which keeps that headroom on the colour side.

Used constants (`cb0` of each stage; reflection, used variables only):

| Shader | Vertex | Pixel |
|---|---|---|
| all | `WorldViewProjection` | `softParticleContrast`, `softParticleEnabled`, `softParticleScale` |
| `vfxadditiveshader` | | also `cameraFadeoutFactor`, `doCameraFadeout` |
| `vfxshapeshader` (zmat) | also `ViewProjection`, `zCrct`, `zMatrix` | |

`WorldViewProjection` and `ViewProjection` are PhyreEngine's derived values
(`docs/formats/scene.md`, "Constants"); `zMatrix`/`zCrct` are the z-matrix
correction (`VFXDynamicGeometry_setZCorrectionValue` `0x006a3b20`); the
soft-particle and camera-fade values are material parameters of the VFX
shaders (where they are written: open question 3). The billboard
particles' positions in the arrays are camera-facing quads with one z for
all four corners (the fill requests carry `g_pppFillMatrix`,
`docs/formats/render.md`); which matrix `WorldViewProjection` then holds
for them is for round 2 to check against the frame's camera.

## Instance keys and makers

A geometry is found in the table by its 64-bit instance key, `(lo, hi)` as
the setup calls pass it (`graphicVFXGetRenderDataByInstance(lo, hi)`); a
several-part effect keeps part `i` at `lo + 10000 i`. The key says who made
it:

| Key | Maker | Setup callers |
|---|---|---|
| `(g_pppCurrentProgram, object)` | a ppp program of the ppp object at `hi` (`pppRunPartStd` `0x00716c40`) | `pppDrawShape0` `0x00740a70` and the other ppp draws (`InitVFXRenderData`, `…Shp`, `…ShapeField`, `…Line`, `…Path3…`) |
| `(0, record)` | the op effect system: the op record at `hi` (0x50-byte records in `FFX.exe`'s op area after `g_Op3dListener` `0x0113fcd0`, or the heap) | `opp_md` `0x008020a0`, `opp_anm` `0x00811120`, `opp_anmm` `0x008124b0`, `opp_anm_yodare` `0x00811bc0`, `opus_0283_draw` `0x00a76770`, `opus_0582_*`, `opu_mdc_break` `0x00804270`, `opp_md_break_init_a74410` |
| fixed pairs | `opus_0706_noise_draw` `(0x15, …)`, `opus_0706_noise_update`/`_acc_update` `(1|2, 0x2c2)`, `opus_0154_wave_draw` `(b, 0x9a)`, `setPolyTris0434`/`setPolyQuad0434` `(0|1, id)`, `AtelExecDrawShape`, `rcbgGuideUpdateWalk`, `rcefModelProc`, … | |

For a ppp key the census walks object `+8` (its birth record) `+0` (the
program list), entry `index` (`+0x28 + 0x10 index`, count `+0x26`) to the
`g_pppProgramTable` record. A list inside a spell overlay is read from
the overlay's file (`magicFiles/FFX/magic_<id>.dll`, mapped at the base in
`g_MagicFile_Handle`; snapshots leave modules out): on disk an entry's
first word is the table index, which the loader turns into the record's
address (magic 481's shape part: `pppPoint`, `pppAngle`, `pppScale`,
`pppColor`, `pppMatrixXYZ`, `pppDrawMatrixFront`, `pppDrawShape`,
`pppVertexAp`).

Makers over the eight frames:

| Frame (overlay) | Effect draws | ppp | op | other |
|---|---|---|---|---|
| battle-magic-1 (481) | 192 | 189 | 3 | 0 |
| battle-magic-2 (481) | 206 | 188 | 18 | 0 |
| battle-magic-3 (481) | 353 | 349 | 3 | 1 |
| battle-overdrive-2 (390, Spiral Cut) | 37 | 4 (object in the overlay's image) | 32 | 1 |
| battle-overdrive-3 (595, Valefor's entry) | 21 | 0 | 21 | 0 |
| battle-overdrive-4 (595) | 21 | 0 | 21 | 0 |
| battle-overdrive-6 (604, the overdrive) | 75 | 42 | 33 | 0 |
| battle-overdrive-7 (604) | 137 | 110 | 27 | 0 |
| **all** | **1042** | **882** | **158** | **2** |

Of the 882 ppp draws, 92 name a drawing program (`pppKeLnsLp`, the
lens-flare loop, in the overdrive); for the other 786 the key's index
names a program without a draw or runner (`pppVertexAp`, `pppVertexApLc`,
`pppRandUpFV`): the draws come from that ppp object, but which program of
it made the instance is not settled (open question 1). The census marks
them `NO-DRAW`.

## A spell live: Fire (ffx-remix M9, 2026-11-07)

A trace in the 32-bit game: ffx-remix's runtime with `FFXREMIX_FX_TRACE=1`
(`Effects/EffectTrace.cs`), harness lockstep, fast mode, virtual clock,
seed 1. Lulu alone against the Thunder Plains' first formation (battle
field 34, formation 0: Larva, actor 20, and Gold Element, actor 21) casts
Fire (magic `0x4c`, `magic_0076.dll`) on the Larva. The runs' probe raised
the monsters' HP to 99999; its writes meant for Lulu's HP landed at her
`+0x674` and `+0x6b0`, inside `limits` (`+0x670`). Hooks on
`FFXApplication_animate` (one count per update; the magic ID and handle
read at the start of each update, before the game's own),
`PhyreScene_DrawMostVFX` (each list entry, its geometry through the table
and its instance key, labelled with the update count of the moment),
`MsSetDamage` (logged when it returns), `MsNumberRegist` and
`MsEffectReady`. Counting from u0, the first update that starts with
`g_MagicFile_MagicId` = `0x4c` (it is set during update u0 - 1):

| Update | What |
|---|---|
| u0 | starts with the magic ID `0x4c` |
| u0 + 1 | starts with `g_MagicFile_Handle` valid |
| u0 + 50 | `MsEffectReady(0x205, 1)` (kind 2, actor 5) |
| u0 + 79 | the spell's own first draws (draws labelled with this count) |
| u0 + 89 | `MsNumberRegist(0x14, …, 170, …)` and `MsSetDamage(5, -5, 0)`: the hit and its damage number |
| u0 + 93 | `MsSetDamage(5, -5, 0)` again |
| u0 + 129 | the spell's own last draws |
| u0 + 138 | `MsSetDamage(5, -5, 0x400)` (the command's end); the magic ID is -1 at the start of u0 + 139 |

Seven traced runs (ffx-remix `m9/probe` trace1, trace2, live1, live2,
exit-vanilla, exit-fire, exit-fire2) had the same updates for the calls;
the draw and handle rows are from the six after trace1 (its tracer did not
yet read the table through the pointer or log the handle's change). The
runs with ffx-remix's hide (live1, live2, exit-fire, exit-fire2) log the
spell's draws too: the tracer reads the list before the hide zeroes it.

The hit most likely comes from the overlay itself (inferred: the tracer
did not log the caller): `magic_0076.dll` has one call to `MsSetDamage`
through `g_MagicImports` slot 239 (`mov eax, [imports+0x3bc]` at
`0x100064b9`, arguments `(…, -5, 0)`), in a timer routine that steps 1/30
a call and calls it at its stages' thresholds 0.35 and 0.5, which fits two
calls 4 updates apart (u0 + 89, u0 + 93). `FFX.exe` has one other caller
with the constants `(…, -5, 0)`, `MsEffectSignalDamage_007b7680` (reached
through `0x007b74d0`, from the motions' effect signals and native
`0x709e`); the `0x400` call at u0 + 138 has no such site in the overlay;
the only `MsSetDamage` call in `FFX.exe` passing a constant `0x400` is
`MsBattleProcess_ChrDamage`'s (the call at `0x007912cb`, `push 0x400` at
`0x007912c3`) (`docs/formats/battle.md`; the tracer did not log the
caller). That `MsNumberRegist` runs inside that `MsSetDamage` is inferred
from the log (the number's line before the call's, which is written as it
returns, in the same update). Hiding all of the spell's draws (below) left
every row above on the same update, the target's HP changed on the same
harness frame, and frame-by-frame shots showed the number first on the
same shot (exit-vanilla against exit-fire and exit-fire2).

**Which draws are the spell's.** In the 142 drawn frames from u0 - 1 to
u0 + 140 (the tracer's window: it logs three frames with the ID -1; the same
in each of the six runs that read the table), 4,680 entries are the
spell's: ppp keys (`(g_pppCurrentProgram, object)`) whose object, and its
birth record's program list (object `+8`, `+0`), lie inside the overlay's
image (the module handle up to `SizeOfImage`); the two tests never
disagree, and all of them use the overlay's textures
(`magic/magic_0076/tex/...`). The other draws of those frames: shared
battle effects, ppp objects outside the image with
`yonishi_data/dat_et/bat_eff/et_tex` textures (20-62 a frame; 21-31 while
the spell's own draws are on screen); 2-5 untextured ppp draws a frame
outside the image (texture name `NoTexture`; flags 2 or 3, programs 4, 5
and 12); two entries in all (at u0 - 1 and u0) with the texture name
"empty"; and the battle map's field programs (`btlmap/kami/kami00_a/fp`
textures). No op-record (`(0, record)`) draws.

**List entries missing from the table.** In each of the six runs, 83 of
the 142 frames have 1-16 entries of `+0x38cc` whose mesh instance is in no
table entry (329 entries, 212 addresses, 154 of them in one frame only).
`graphicVFXDestroy` takes a geometry out of the table at once
(`ClassVFXRenderDataTable_Remove_0069fc10`: the map node erased, the
geometry to the delete pool), but leaves its list entry: its virtual
`+0x10` (`VfxRenderData_pushPair_006a0f50`) only queues the mesh instance
on scene `+0x3894`, and `PhyreScene_doDeleteMeshInstances` (`0x0065a620`)
zeroes the entry (`PhyreScene_removeVFXFromRenderArray`) after the frame's
render. The entries missing from the table are most likely instances
between those two steps. A hide by instance key therefore remembers the
spell's entries while the spell plays: in exit-fire2 ffx-remix's hide
zeroed 4,891 entries (a running count, Fire's alone in that run) against
the 4,680 the tracer resolved as the spell's; the other 211 are most
likely remembered entries whose geometry had left the table (exit-fire2
has 224 "?" entries from u0 + 79 to u0 + 140); the count also covers one
frame after the tracer's window. The remembered addresses must be dropped
a few frames after the spell: the game reuses them for other effects'
geometry (an earlier version that kept them hid later effects' draws).

**Zeroing list entries** of `+0x38cc` (count `+0xf8cc`) before
`DrawMostVFX` runs (and putting them back after) hides those draws: the
loop skips zero entries and the draw events still run at their places
(read from the code: no Fire frame had an event).

**Where the actors are.** The caster is the actor with a command cue entry
(`+0xde5` not `0xff`) and an effect state (`+0xdfb` not 0; sampled 16, 2,
4, 5 over Fire; 16 appears nowhere in battle.md's `MsEffectStart` section,
3 and 11 were not sampled); its targets are the bits of its `+0xdc4`
(`docs/formats/battle.md`). A battle character's place in the HD scene
(the space the draws' world matrices use) was taken as `(-x, -y, z)` of
`Ch_GetPos` (chr `+0xc`), as on field maps: an effect placed there sat on
the Larva in the live frames (checked by eye, run live3). The centre of
the character's mesh bounds (the render instance at chr `+0x830`, each
mesh instance's box) is not that place: in live2 it lay 31.8 units from it
for the Larva, 51.4 for the Gold Element and 55.4 for Lulu, and an effect
placed by it stood on the ground beside the Larva.

## Gate

The M4 census: every effect draw of the eight captured frames is traced:
1042 of 1042, to the `DrawMostVFX` list place it is drawn from (1018 by
the list entry, 24 by a zeroed entry and the delete pool), the geometry
and segment whose bytes it draws, the instance key and its maker (ppp
object or op record), and the shader its flags give (`cargo test -p
ffx-fxdraw --test captures`: "8 frames, 1042 effect draws traced"; no
unexplained draw of any class in the frames). `tests/synthetic.rs` checks
the census itself on a synthetic frame.

## Open questions

1. **Which ppp program made the instance.** (Which *spell* made it: see "A
   spell live: Fire".) The key's low word is
   `g_pppCurrentProgram`, but for 786 draws it names `pppVertexAp`,
   `pppVertexApLc` or `pppRandUpFV`, which have no draw (`pppVertexAp`
   births particles at vertices, `pppBornVertex`). Either the born objects
   draw with the parent's index left in `g_pppCurrentProgram`, or the draws
   run later (the OT list, `pppOtFunc_Control` `0x00716ac0`). A coverage
   run of `pppDrawShape0` with the program index and object logged settles
   it.
2. **The blend factors.** Flags `0xa` draw with source factor one or
   source alpha, flags `0x8` with source alpha or one; the geometry's words
   `+0x30..+0x5c` do not tell them apart. Candidates: the alpha function
   (`calculateVFXSP_AlphaFunction`, `g_AlphaScale`), the material's
   blend set by `ClassDynamicGeometry_setupMaterial` (`0x006a3b50`).
3. **The VFX material constants** (`softParticle*`, `cameraFadeout*`,
   `zCrct`): who writes them and when.
4. **The other lists**: no frame used `+0x3880` (no draw pass found for
   it), the DTT, UI 2D or after-post lists; the field capture's
   `vfxnzadditiveshadernt` draw after the UI comes from one of them. Field
   and event effects and the spells with draw events (copy buffer, screen
   fade, character draws) need captures of their own.
5. **The op records**: name the op system's record layout (0x50 bytes,
   `FFX.exe` area after `g_Op3dListener`) and which `opp_*` function each
   record belongs to; the census stops at the record.
6. **The late world** (scene `+0x9c` and the post passes): the scene
   builder's, with the summon's character drawn after the glow.
7. **PC2 captures** use other saves and timing (above); the main
   machine should take the git scripts' captures and run the gate on them.
